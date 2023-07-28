use std::{
    env, fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;

use arecibo::extract::{extract, extract_text, find_links};
use arecibo::util::slice_up_to;
use url::Url;

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
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let _ = conn.execute(
            "INSERT INTO url (url, discovered) VALUES (?1, ?2)",
            (&url, now),
        );
    }

    for url in env::args().skip(1) {
        println!("Adding {} to the list of URL's to crawl", url);
        insert(&conn, &url);
    }

    // Let's go crawl!
    let mut find_to_crawl =
        conn.prepare("SELECT url FROM url WHERE crawled IS NULL ORDER BY discovered ASC LIMIT 1")?;
    let mut delete_url = conn.prepare("DELETE FROM url WHERE url = ?1")?;
    loop {
        let mut rows = find_to_crawl.query(())?;
        let mut found_some = false;
        while let Some(row) = rows.next()? {
            let url: String = row.get(0)?;
            println!("Found url {}", url);
            found_some = true;
            delete_url.execute(&[&url])?;

            // Let's crawl.
            let body = ureq::get(&url).call()?.into_string()?;
            let mut body_slice = slice_up_to(&body, 1024 * 250).as_bytes();

            let url = Url::parse(&url).unwrap();
            // TODO: main page of wikipedia does not extract correctly. Firefox reader works.

            let (dom, cleaned_document) = extract(&mut body_slice, &url);
            let mut clean: String = String::new();
            extract_text(&cleaned_document, &mut clean, true);

            println!("{}", clean);

            let mut links = Vec::new();
            find_links(&cleaned_document, &mut links);
            for link in links {
                println!("{:?}", link);
                insert(&conn, &link.href);
            }
        }
        if !found_some {
            break;
        }
    }
    Ok(())
}
