use markup5ever_rcdom::Handle;
use markup5ever_rcdom::NodeData::{Element, Text};
use markup5ever_rcdom::RcDom;
use readability::scorer;
use readability::scorer::Candidate;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;
use url::Url;

// Modified from https://github.com/kumabook/readability/blob/master/src/dom.rs
pub fn extract_text(handle: &Handle, text: &mut String, deep: bool) {
    for child in handle.children.borrow().iter() {
        let c = child.clone();
        match c.data {
            Text { ref contents } => {
                text.push_str(contents.borrow().trim());
            }
            Element { .. } => {
                if deep {
                    extract_text(child, text, deep);
                }
            }
            _ => (),
        }
        let last_char = text.chars().last();
        if last_char != Some(' ') && last_char != None {
            text.push(' '); // To make sure we get spaces.
        }
    }
}

#[derive(Debug)]
pub struct Link {
    pub href: String,
    pub title: String,
}

pub fn find_links(handle: &Handle, links: &mut Vec<Link>) {
    for child in handle.children.borrow().iter() {
        let c = child.clone();
        match &c.data {
            Element { name, attrs, .. } => {
                if name.local.to_string() == "a" {
                    // print!("<{}", name.local);
                    for attr in attrs.borrow().iter() {
                        if attr.name.local.to_string() == "href" {
                            // print!(" {}=\"{}\"", attr.name.local, attr.value);
                            let mut title: String = String::new();
                            extract_text(&c, &mut title, true);
                            links.push(Link {
                                href: attr.value.to_string(),
                                title,
                            })
                        }
                    }
                    // println!(">");
                }
                find_links(child, links);
            }
            _ => (),
        }
    }
}

pub fn extract(mut dom: &mut RcDom, url: &Url) -> (Rc<markup5ever_rcdom::Node>, String) {
    let mut title = String::new();
    let mut candidates = BTreeMap::new();
    let mut nodes = BTreeMap::new();
    let handle = dom.document.clone();
    scorer::preprocess(&mut dom, handle.clone(), &mut title);
    scorer::find_candidates(
        &mut dom,
        Path::new("/"),
        handle.clone(),
        &mut candidates,
        &mut nodes,
    );
    let mut id: &str = "/";
    let mut top_candidate: &Candidate = &Candidate {
        node: handle.clone(),
        score: Cell::new(0.0),
    };
    for (i, c) in candidates.iter() {
        let score = c.score.get() * (1.0 - scorer::get_link_density(c.node.clone()));
        c.score.set(score);
        if score <= top_candidate.score.get() {
            continue;
        }
        id = i;
        top_candidate = c;
    }
    let node = top_candidate.node.clone();
    scorer::clean(&mut dom, Path::new(id), node.clone(), url, &candidates);

    (node, title)
}
