use std::io::Read;
use std::{
    env, fs,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use html5ever::{parse_document, tendril::TendrilSink};
use markup5ever_rcdom::RcDom;
use rusqlite::Connection;

use arecibo::extract::{extract, extract_text, find_links};
use arecibo::util::slice_up_to;
use url::{ParseError, Url};

fn main() -> anyhow::Result<()> {
    fs::create_dir_all("store")?;
    let conn = Connection::open("store/crawl.sqlite")?;

    // Create DB structure
    conn.execute(
        "CREATE TABLE IF NOT EXISTS url (
            url TEXT NOT NULL,
            discovered INTEGER NOT NULL,
            crawled INTEGER
        )",
        (),
    )?;
    conn.execute(
        "
        CREATE INDEX IF NOT EXISTS url_find_to_crawl on url(crawled, discovered)
    ",
        (),
    )?;
    conn.execute(
        "
        CREATE UNIQUE INDEX IF NOT EXISTS find_exists on url(url)
    ",
        (),
    )?;

    fn insert(conn: &Connection, url: &str) {
        if let Ok(_parsed) = Url::parse(url) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if conn
                .execute(
                    "INSERT INTO url (url, discovered) VALUES (?1, ?2)",
                    (&url, now),
                )
                .is_ok()
            {
                println!("Discovered {}", url);
            }
        } else {
            println!("Ignoring invalid URL: {}", url);
        }
    }

    fn join_if_needed(base: &Url, input: &str) -> anyhow::Result<Url> {
        match Url::parse(input) {
            Ok(url) => Ok(url),
            Err(ParseError::RelativeUrlWithoutBase) => Ok(base.join(input)?),
            error => Ok(error?),
        }
    }

    fn clean_url(base: &Url, input: &str) -> anyhow::Result<String> {
        let url = join_if_needed(base, input)?;

        let scheme = url.scheme();
        let host = url.host().context("no host")?.to_string();
        let path = url.path();
        Ok(if let Some(port) = url.port() {
            format!("{}://{}:{}{}", scheme, host, port, path)
        } else {
            format!("{}://{}{}", scheme, host, path)
        })
    }

    for url in env::args().skip(1) {
        println!("Adding {} to the list of URL's to crawl", url);
        insert(&conn, &url);
    }

    // Let's go crawl!
    fn agent() -> ureq::Agent {
        ureq::AgentBuilder::new().user_agent("Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko; compatible; Arecibot/0.1; https://localhost/todo/fill/this/in) Chrome/115.0.0.0 Safari/537.36").build()
    }

    let mut find_to_crawl = conn.prepare(
        "SELECT url, discovered FROM url WHERE crawled IS NULL ORDER BY discovered ASC LIMIT 1",
    )?;
    let mut delete_url = conn.prepare("DELETE FROM url WHERE url = ?1")?;
    loop {
        let mut rows = find_to_crawl.query(())?;
        let mut found_some = false;
        while let Some(row) = rows.next()? {
            let url: String = row.get(0)?;
            let discovered: u64 = row.get(1)?;
            found_some = true;
            delete_url.execute(&[&url])?;

            // Let's crawl.
            let mut body = agent().get(&url).call()?.into_reader().take(1024 * 250);

            let url = Url::parse(&url).unwrap();
            // TODO: main page of wikipedia does not extract correctly. Firefox reader works.

            let mut dom = parse_document(RcDom::default(), Default::default())
                .from_utf8()
                .read_from(&mut body)
                .unwrap();

            let mut links = Vec::new();
            find_links(&dom.document, &mut links);
            for link in links {
                if let Ok(link_url) = clean_url(&url, &link.href) {
                    insert(&conn, &link_url);
                } else {
                    println!("Invalid URL for {:?}", link);
                }
            }

            let cleaned_document = extract(&mut dom, &url);
            let mut clean: String = String::new();
            extract_text(&cleaned_document, &mut clean, true);

            println!("");
            println!("{} {}", url, discovered);
            println!("{}", clean);

            std::thread::sleep(Duration::from_secs(2));
        }
        if !found_some {
            break;
        }
    }
    Ok(())
}
