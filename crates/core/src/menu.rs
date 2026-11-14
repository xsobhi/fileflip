//! Platform-independent model of the "Convert to" context menu.
//! The DLL turns this tree into real Win32 popup menus.

use crate::formats::{ext_matches_format, ext_of, Category, Format};
use crate::request::Preset;
use crate::settings::Settings;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Convert(Preset),
    Custom,
    Settings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Item { label: String, action: Action },
    Sub { label: String, children: Vec<Node> },
    Separator,
}

impl Node {
    fn item(label: impl Into<String>, action: Action) -> Node {
        Node::Item { label: label.into(), action }
    }
    fn sub(label: impl Into<String>, children: Vec<Node>) -> Node {
        Node::Sub { label: label.into(), children }
    }
}

/// What is selected in Explorer, grouped by category.
#[derive(Default, Debug)]
pub struct Selection {
    pub files: Vec<PathBuf>,
    pub by_cat: BTreeMap<Category, (usize, BTreeSet<String>)>,
}

impl Selection {
    pub fn new(paths: impl IntoIterator<Item = PathBuf>, settings: &Settings) -> Selection {
        let mut s = Selection::default();
        for p in paths {
            let ext = ext_of(&p);
            if let Some(c) = crate::formats::category_of_ext(&ext) {
                if !settings.menu_enabled(c) {
                    continue;
                }
                let e = s.by_cat.entry(c).or_default();
                e.0 += 1;
                e.1.insert(ext);
                s.files.push(p);
            }
        }
        s
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Build the whole menu. Returns `None` when nothing selected is convertible.
pub fn build(sel: &Selection) -> Option<Node> {
    if sel.is_empty() {
        return None;
    }
    let mut top = Vec::new();
    if sel.by_cat.len() == 1 {
        let (&cat, (_, exts)) = sel.by_cat.iter().next().unwrap();
        top.extend(category_items(cat, exts));
        top.push(Node::Separator);
        top.push(Node::item("Custom…", Action::Custom));
    } else {
        for (&cat, (count, exts)) in &sel.by_cat {
            top.push(Node::sub(format!("{} ({count})", cat.plural()), category_items(cat, exts)));
        }
        top.push(Node::Separator);
        top.push(Node::item(format!("Custom… (all {} files)", sel.files.len()), Action::Custom));
    }
    top.push(Node::item("Settings…", Action::Settings));
    Some(Node::sub("Convert to", top))
}

fn preset(target: &Format, cat: Category) -> Preset {
    Preset { only: Some(cat), ..Preset::new(target.id) }
}

fn category_items(cat: Category, exts: &BTreeSet<String>) -> Vec<Node> {
    let mut items = Vec::new();
    let all_same = |f: &Format| !exts.is_empty() && exts.iter().all(|e| ext_matches_format(e, f));

    for f in cat.targets() {
        if !all_same(f) {
            items.push(Node::item(f.label, Action::Convert(preset(f, cat))));
        }
    }

    if cat == Category::Video {
        let audio: Vec<Node> = Category::Audio
            .targets()
            .map(|f| Node::item(f.label, Action::Convert(preset(f, cat))))
            .collect();
        items.push(Node::Separator);
        items.push(Node::sub("Audio only", audio));
    }

    let mut more = Vec::new();
    for f in cat.targets() {
        let opts = crate::presets::more_options(f, cat);
        if !opts.is_empty() {
            more.push(Node::sub(f.label, opts));
        }
    }
    if !more.is_empty() {
        items.push(Node::Separator);
        items.push(Node::sub("More options", more));
    }
    items
}

/// Flatten the leaves in depth-first order; the DLL uses the index as command id.
pub fn leaves(node: &Node, out: &mut Vec<Action>) {
    match node {
        Node::Item { action, .. } => out.push(action.clone()),
        Node::Sub { children, .. } => children.iter().for_each(|c| leaves(c, out)),
        Node::Separator => {}
    }
}

/// Render as indented text (used by `fileflip debug-menu` and tests).
pub fn render(node: &Node) -> String {
    fn go(n: &Node, depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth);
        match n {
            Node::Item { label, action } => {
                let what = match action {
                    Action::Convert(p) => p.args().join(" "),
                    Action::Custom => "custom".into(),
                    Action::Settings => "settings".into(),
                };
                out.push_str(&format!("{pad}{label}    [{what}]\n"));
            }
            Node::Sub { label, children } => {
                out.push_str(&format!("{pad}{label} >\n"));
                children.iter().for_each(|c| go(c, depth + 1, out));
            }
            Node::Separator => out.push_str(&format!("{pad}----\n")),
        }
    }
    let mut s = String::new();
    go(node, 0, &mut s);
    s
}
