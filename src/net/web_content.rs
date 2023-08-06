/*
   Copyright 2023 Krol Inventions B.V.

   This file is part of DawnSearch.

   DawnSearch is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   DawnSearch is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with DawnSearch.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::time::Duration;

use crate::{search::search_provider::SearchResult, util::slice_up_to};

/**
 * Who needs a templating engine when you've got format!?
 */

pub fn page(title: &str, body: &str) -> String {
    format!(
        r#"
<html>
<head>
<title>{title}</title>
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<style>
        body {{
            font-family: 'Open Sans Light','sans-serif', sans-serif;
            width: 100%;
            height: 100%;
            padding-bottom: 1em;
            margin: 0;
        }}
        h1 {{
            font-size: min(13vw, 4em);
            margin-block-end: 0em;
            color: #4f009f;      
            font-weight: 300;
            /* To get the light font weight to work on Chrome. */
            font-family: 'Open Sans Light','sans-serif', sans-serif;
        }}
        .tagline {{
            margin-left: 2em;
            margin-right: 2em;
        }}
        h1.small {{
            font-size: 2em;
            margin-top: 0.2em;
        }}
        h1.small > a {{
            text-decoration: none;
            color: #4f009f;
        }}
        h3 {{
            margin-block-end: 0.5em;
        }}
        p {{
            margin-block-start: 0.5em;
        }}
        .index-page {{
            display: flex;
            align-items: center;
            flex-direction: column;
            position: relative;
            top: 20%;
            width: 100%;
            padding-bottom: 2em;
        }}
        @media (max-width: 700px) {{
            .index-page {{
                top: 0%;
            }}
            .search {{
                flex-wrap: wrap;
                justify-content: flex-end;
            }}
        }}
        .index-header {{
            display: flex;
            align-items: center;
            flex-direction: column;
            width: 100%;
        }}
        input,textarea{{width:100%;display:block}}
        .search {{
            display: flex;
            width: 90%;
            max-width: 800px;
            gap: 0.5em;
            margin-left: 2em;
            margin-right: 2em;
        }}        
        .search > input {{
            font-size: 1.2em;
            border-radius: 0.3em;
            padding: 0.5em;
        }}
        .search-input {{
        }}
        .search-button {{
            width: 6em;
        }}
        .description {{
            max-width: 800px;
            color: #6c6375;
            border: 1px #e5d9e9 solid;
            border-radius: 1em;
            padding: 1.3em;
            padding-top: 0.6em;
            padding-bottom: 0.6em;
            background-color: #f9f9f9;
            margin-top: 1.5em;
        }}
        .top-search {{
            display: flex;
            width: 100%;
            border-bottom: 1px solid #c3c3c3;
            background-color: #f9f9f9;
            column-gap: 0;
            padding-top: 1em;
            padding-left: 1em;         
        }}
        .result.exploring {{
            background-color: #f9f9f9;
            padding: 1em;
            border-radius: 8px;
            border: solid 1px #d5d5d5;
            margin-left: -1em;
            margin-right: -1em;
            padding-bottom: 0;
            margin-bottom: 2em;            
        }}
        .results {{
            margin-left: 258px;
            margin-right: 1.3em;
            max-width: 800px;
            font-size: 90%;
            color: #141414;
        }}
        .result-url {{
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            max-width: 760px;
            display: block;
        }}
        .result-title {{
            font-size: 1.3em;
            overflow: hidden;
            text-overflow: ellipsis;
            max-width: 760px;
            display: block;
        }}
        .result-top {{
            color: #9f9ba5;
        }}
        .currently-exploring {{
            display: none;
            color: 4f009f;
        }}
        .result.exploring > .result-top {{
            display: none;
        }}
        .result.exploring > .currently-exploring {{
            display: block;
        }}
        .result-text {{
            margin-bottom: 1.4em;
            margin-top: 0.4em;
        }}
        .result-explore {{
            background-color: #c1c1c1;
            color: white;
            border-radius: 3px;
            padding: 2px;
            padding-left: 5px;
            padding-right: 5px;
            font-size: 90%;
            text-decoration: none;
        }}
        .result-explore:hover {{
            background-color: #8350ff;
        }}

        @media (max-width: 1060px) {{
            .search {{
                width: 95%;
            }}
            .top-search {{
                flex-wrap: wrap;
            }}
            .results {{
                margin-left: 1.3em;
            }}
        }}        
</style>
</head>
<body>
{body}
</body>
</html>
"#
    )
}

pub fn main_page() -> String {
    let s = search_box("");
    page(
        "DawnSearch",
        &format!(
            r#"
<div class="index-page">
<div class="index-header">
<h1 class="title">DawnSearch</h1>
<p class="tagline">
    The open source distributed web search engine that searches by meaning
</p>
</div>
{s}
<script>
    document.getElementById("searchbox").focus();
</script>   
<div class="description">
<h3>What is DawnSearch?</h3>
<p>
    The DawnSearch project has as goal to build a new searching paradigm.
    No longer centralized, controlled by big corporations, but a more human kind of search, one that values discovery and inspiration.
</p>
<ul>
<li>
    DashSearch is distributed, which means it does not run on a single server, or even on a single continent. Volunteers from all over the world can host their own DawnSearch instance,
    which will then connect to the others to form a global search engine.
</li>
<li>
    DawnSearch is open, which means that everyone can take the source, and do with it whatever they want. Development is done in an open community, where every voice is valuable and is heard.
</li>
<li>
    DawnSearch does not search for the words you type in directly. An AI model will read your search query and convert
    it into a list of 384 numbers, which you can consider a location in a 384-dimensional space. All documents in DawnSearch have also been analyzed and given a location.
    We then simply look for the documents which are closest to your query.
</li>
</ul>
<h3>Privacy</h3>
<p>
This DawnSearch instance does not actively collect data on access, and does not store searches. However, some information may be temporarily stored in log files. Due to the way DawnSearch works, a processed form of your 
seach query is sent to other instances. <b>Do not use DawnSearch to search for any sensitive information.</b>
</p>
<h3>Does this work as well as Google, Bing, Brave Search etc?</h3>
<p>Currently, no. DawnSearch has just 0.1% of the data of one of a big dataset loaded. And this is still only a part of the internet. Over the next coming months the index will expand, and we will have to discover
what that does to the quality of the results. As DawnSearch is an experiment, we hope to find a lot of improvments still.
</p>
<h3>AI and statistics</h3>
<p>
DawnSearch uses AI and statistical techniques in order to search. This does mean that biases may be present.
For example, certain kinds of language use may not be detected as 'English' and would then be excluded from the index.
The AI model used, <a href="https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2">all-MiniLM-L6-v2</a>, may also prefer certain content over others. This is currently unknown.
For example, it could decide it likes pages written by a male author more than written by a female. These biases may come from the training data itself, or it may happen because the AI
is not human and thinks differently than we do.
</p>
<h3>Open Source / Free Software</h3>
<p>
The code for this instance is available on <a href="https://github.com/dawn-search/dawnsearch">GitHub</a> under an open source / free software license. In short, anyone is free to modify this software, with the important note that
if they give other people access, they will also have to share their modifications with them.
</p>
</div>
</div>
    "#
        ),
    )
}

pub fn results_page(search_query: &str, results: &str) -> String {
    let s = search_box(search_query);
    let title = format!("{} - DawnSearch", search_query);
    page(
        &title,
        &format!(
            r#"
<div class="top-search">
 <h1 class="small"><a href="/">DawnSearch</a></h1>
{s}
</div>
<div class="results">
{results:}
</div> 
    "#
        ),
    )
}

fn search_box(search_query: &str) -> String {
    let s = html_escape::encode_double_quoted_attribute(search_query);
    format!(
        r#"
    <form method="get" class="search">
    <input name="q" id="searchbox" class="search-input" value="{}">
    <input type="submit" value="Explore" class="search-button">
</form>
"#,
        s
    )
}

pub fn format_results(result: &SearchResult, elapsed: Duration) -> String {
    let mut r = String::new();
    r += &format!(
        "<p>Searched {} pages on {} instances in {:.2} seconds</p>",
        result.pages_searched,
        result.servers_contacted + 1,
        elapsed.as_secs_f32()
    );
    for result in &result.pages {
        let url_encoded_u = html_escape::encode_double_quoted_attribute(&result.url);
        let url_encoded = html_escape::encode_text(&result.url);
        let title_encoded = html_escape::encode_text(&result.title);
        let s = slice_up_to(&result.text, 400);
        let text_encoded = html_escape::encode_text(s);
        let distance = if result.distance < 0.0 {
            0.0
        } else {
            result.distance
        }; // Prevent -0.0 from showing up.
        let explore = format!(
            r#"<a href="?s={}:{}" title="Find pages like this one" class="result-explore">explore</a>"#,
            result.instance_id, result.page_id
        );
        let exploring = if result.distance < 0.001 {
            "exploring"
        } else {
            ""
        };
        r += &format!(
            r#"
<div class="result {exploring}"><div class="currently-exploring">Exploring</div>
<div class="result-top">{:.2} {explore} <i class="result-url">{}</i></div>
<div class="result-title"><a href="{}">{}</a></div>
<div class="result-text">
    {}...
</div>
</div>
"#,
            distance, url_encoded, url_encoded_u, title_encoded, text_encoded,
        );
    }
    r
}
