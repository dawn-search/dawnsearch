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
        * {{-webkit-font-smoothing: antialiased;}}
        body {{
            font-family: 'Open Sans Light','sans-serif';
            width: 100%;
            height: 100%;
            padding-bottom: 1em;
        }}
        h1 {{
            font-size: min(13vw, 4em);
            margin-block-end: 0em;
            color: #4f009f;      
            font-weight: 300;
            /* To get the light font weight to work on Chrome. */
            font-family: 'Open Sans Light','sans-serif';
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
            max-width: 700px;
            gap: 0.5em;
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
    let s = search_box();
    page(
        "DawnSearch",
        &format!(
            r#"
<div class="index-page">
<div class="index-header">
<h1 class="title">DawnSearch</h1>
<p class="tagline">
    Open source distributed web search engine, that searches by meaning
</p>
</div>
{s}
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
<h3>Does this work as well as Google, Bing, Brave Search etc?</h3>
<p>Currently, no. DawnSearch has just 0.1% of the data of one of a big dataset loaded. And this is still only a part of the internet. There is just so much information! Over the next coming months the index will expand, and we will have to discover
what that does to the quality of the results! As DawnSearch is an experiment, we hope to find a lot of improvments still.
</p>
<h3>AI and statistics</h3>
<p>
DawnSearch uses AI and statistical techniques in order to search. This does mean that biases may be present.
For example, certain kinds of language use may not be detected as 'English' and would then be excluded from the index.
The AI model used, <a href="https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2">all-MiniLM-L6-v2</a>, may also prefer certain content over others. This is currently unknown.
For example, it could decide it likes pages written by a male author more than written by a female. These biases may come from the training data itself, or it may happen because the AI
is not human and thinks differently than we do.
</p>
</div>
</div>
    "#
        ),
    )
}

pub fn results_page(results: &str) -> String {
    let s = search_box();
    page(
        "DawnSearch",
        &format!(
            r#"
 <h1>DawnSearch</h1>
{s}
{results:}
<script>
document.getElementById("searchbox").focus();
</script>    
    "#
        ),
    )
}

fn search_box() -> String {
    format!(
        r#"
    <form method="get" class="search">
    <input name="q" id="searchbox" class="search-input">
    <input type="submit" value="Explore" class="search-button">
</form>
<script>
document.getElementById("searchbox").focus();
</script>
"#
    )
}
