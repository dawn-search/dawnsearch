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
use url::{ParseError, Url};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let restart = args.contains(&"--restart".to_owned());

    fs::create_dir_all("store")?;
    let conn = Connection::open("store/crawl.sqlite")?;
    conn.busy_timeout(Duration::from_millis(10000))?;

    // Create DB structure
    conn.execute(
        "CREATE TABLE IF NOT EXISTS page (
            host TEXT NOT NULL,
            path TEXT NOT NULL,
            discovered INTEGER NOT NULL,
            crawled INTEGER
        )",
        (),
    )?;
    conn.execute(
        "
        CREATE INDEX IF NOT EXISTS url_find_to_crawl on page(crawled, discovered)
    ",
        (),
    )?;
    conn.execute(
        "
        CREATE UNIQUE INDEX IF NOT EXISTS find_exists on page(host, path)
    ",
        (),
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS host (
            host TEXT NOT NULL,
            pages INTEGER DEFAULT 0 NOT NULL
        )",
        (),
    )?;
    conn.execute(
        "
        CREATE UNIQUE INDEX IF NOT EXISTS host_unique on host(host)
    ",
        (),
    )?;
    conn.execute(
        "
        CREATE INDEX IF NOT EXISTS find_host on host(pages)
    ",
        (),
    )?;

    if restart {
        conn.execute("UPDATE page SET crawled = NULL", ())?;
        conn.execute("UPDATE host SET pages = 0", ())?;
    }

    fn add_host(conn: &Connection, host: &str) -> anyhow::Result<()> {
        conn.execute(
            "INSERT INTO host (host, pages) VALUES (?1, 0) ON CONFLICT(host) DO NOTHING",
            [&host],
        )?;
        Ok(())
    }
    fn request_for_host(conn: &Connection, host: &str) -> anyhow::Result<()> {
        conn.execute(
            "INSERT INTO host (host, pages) VALUES (?1, 1) ON CONFLICT(host) DO UPDATE SET pages = pages + 1",
            [&host],
        )?;
        Ok(())
    }
    fn timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
    fn add_link(conn: &Connection, url: &str) -> anyhow::Result<()> {
        if let Ok(parsed) = Url::parse(url) {
            if parsed.port().is_some() {
                println!("Ignoring URL with port: {}", url);
                return Ok(());
            };

            // Note that we silently convert all links to HTTPS.
            let host = format!("https://{}", parsed.host().expect("host"));
            let path = parsed.path();

            let now = timestamp();
            let result = conn.execute(
                "INSERT INTO page (host, path, discovered) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                (&host, &path, now),
            )?;
            if result > 0 {
                // println!("Discovered {}", url);
                add_host(&conn, &host)?;
            }
        } else {
            println!("Ignoring invalid URL: {}", url);
        }
        Ok(())
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
        if url.starts_with("--") {
            continue;
        }
        println!("Adding {} to the list of URL's to crawl", url);
        add_link(&conn, &url)?;
    }

    // Let's go crawl!
    fn agent() -> ureq::Agent {
        ureq::AgentBuilder::new()
            .user_agent("Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko; compatible; Arecibot/0.1; https://localhost/todo/fill/this/in) Chrome/115.0.0.0 Safari/537.36")
            .max_idle_connections(0)
            .timeout(Duration::from_secs(2))
            .build()
    }

    let mut find_to_crawl = conn.prepare(
        "SELECT page.host, path, discovered FROM page INNER JOIN host ON page.host = host.host WHERE crawled IS NULL ORDER BY host.pages ASC LIMIT 1",
    )?;
    let mut mark_crawled =
        conn.prepare("UPDATE page SET crawled = ?1 WHERE host = ?2 AND path = ?3")?;
    let mut pages_crawled = 0;
    loop {
        let mut rows = find_to_crawl.query(())?;
        let mut found_some = false;
        while let Some(row) = rows.next()? {
            let host: String = row.get(0)?;
            let path: String = row.get(1)?;
            let discovered: u64 = row.get(2)?;
            found_some = true;

            let now = timestamp();
            mark_crawled.execute((now, &host, &path))?;

            let url = format!("{}{}", host, path);

            // Let's crawl.
            request_for_host(&conn, &host)?;
            let response = match agent().get(&url).call() {
                Ok(r) => r,
                Err(e) => {
                    println!("Failed to load {}: {}", e, url);
                    continue;
                }
            };
            let mut body = response.into_reader().take(1024 * 250);

            let url = Url::parse(&url).unwrap();
            // TODO: main page of wikipedia does not extract correctly. Firefox reader works.

            let mut dom = match parse_document(RcDom::default(), Default::default())
                .from_utf8()
                .read_from(&mut body)
            {
                Ok(dom) => dom,
                Err(e) => {
                    println!("Failed to read {}: {}", e, url);
                    continue;
                }
            };

            let mut links = Vec::new();
            find_links(&dom.document, &mut links);
            for link in links {
                if let Ok(link_url) = clean_url(&url, &link.href) {
                    add_link(&conn, &link_url)?;
                }
            }

            let cleaned_document = extract(&mut dom, &url);
            let mut clean: String = String::new();
            extract_text(&cleaned_document, &mut clean, true);

            pages_crawled += 1;
            println!("{} {} {}", pages_crawled, url, discovered);
            // println!("{}", clean);

            // std::thread::sleep(Duration::from_secs(2));
        }
        if !found_some {
            break;
        }
    }
    Ok(())
}
