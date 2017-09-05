/*
 * list-maildirs
 */
#[macro_use]
extern crate clap;
extern crate walkdir;

use clap::{App, Arg};
use std::env;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir, WalkDirIterator};

const TILDE: &'static str = "~";
const TILDE_SLASH: &'static str = "~/";

// Crude ~ -> $HOME expansion.
// Can possibly use String.replacen here in the future
// Must be a better way than this.
fn expand_path(path: &str) -> PathBuf {
    let mut cut_len = 0;

    if path.starts_with(TILDE) {
        cut_len = TILDE.len();
    }

    if path.starts_with(TILDE_SLASH) {
        cut_len = TILDE_SLASH.len();
    }

    // We can stop here and just return a PathBuf from the path if we're not
    // replacing anything.
    if cut_len == 0 {
        return PathBuf::from(path);
    }

    // Otherwise, continue and replace the ~
    let home = match env::home_dir() {
        None       => panic!("Could not get your home dir."),
        Some(path) => path,
    };

    let pathtmp = &path[cut_len..];

    home.join(&pathtmp)
}

// Filter for entries being a directory.
fn is_dir(entry: &DirEntry) -> bool {
    entry.path().is_dir()
}

// Filter for entry == "cur"
fn is_cur(entry: &DirEntry) -> bool {
    entry.file_name() == "cur"
}

// Filter excluded maildirs
fn is_excluded(entry: &PathBuf, excluded: &Vec<PathBuf>) -> bool {
    excluded.contains(entry)
}

// Checks if a maildir was listed as an initial maildir.
fn is_initial(maildir: &PathBuf, initial: &Vec<PathBuf>) -> bool {
    initial.contains(maildir)
}

fn maildir_path(base: &Path, path: &Path) -> PathBuf {
    // Attempt to get the parent of the path we were given.
    // If we're successful, we strip the base prefix from it.
    let maildir = match path.parent() {
        None    => panic!("No parent directory for {:?}", path),
        Some(x) => x.strip_prefix(&base),
    };

    // If stripping the base prefix was successful, return the maildir path.
    match maildir {
        Err(e) => panic!("{}", e),
        Ok(m)  => m.to_owned(),
    }
}

fn list_maildirs(base: &str,
                 initial: &Vec<PathBuf>,
                 excluded: &Vec<PathBuf>)
                 -> Vec<PathBuf> {
    let base = expand_path(&base);

    // Filter the Maildir into what we're really after.
    // .. get an interator
    // .. grab the directories
    // .. that we can access
    // .. and it's a 'cur' Maildir directory.
    // .. get the maildir path.
    // .. remove our exclusions.
    // .. finally collect the vector of PathBufs
    let dirs = WalkDir::new(&base)
        .into_iter()
        .filter_entry(is_dir)
        .filter_map(Result::ok)
        .filter(is_cur)
        .map(|e| maildir_path(&base, e.path()))
        .filter(|e| !is_excluded(e, excluded))
        .collect::<Vec<PathBuf>>();

    let mut found_initial = Vec::with_capacity(initial.len());
    let mut maildirs = Vec::with_capacity(dirs.len());

    // Go over our maildirs, pushing them into the maildirs vector after
    // stripping off the base prefix.
    for maildir in dirs {
        // Sort mailboxes into two vectors depending if they're an initial
        // maildir or not.
        if is_initial(&maildir, &initial) {
            found_initial.push(maildir.clone());
        } else {
            maildirs.push(maildir.clone());
        }
    }

    // At this point, `found_initial` tells us which initial dirs exist and
    // are actual maildirs. However, the order is all wrong. We want our
    // initial directories to be in the order they were specified on the
    // command line.
    // The `initial` vector is in that order.
    // We generate another vector based on `initial` and `found_initial` that
    // is in the correct order.
    let mut initial_order = Vec::with_capacity(found_initial.len());
    for maildir in initial {
        let pathbuf = PathBuf::from(maildir);
        if found_initial.iter().any(|x| x == &pathbuf) {
            initial_order.push(pathbuf.clone());
        }
    }

    // Sort maildirs that aren't the initial bunch.
    maildirs.sort();

    let mut all = Vec::with_capacity(initial_order.len() + maildirs.len());
    all.extend(initial_order);
    all.extend(maildirs);
    all
}

fn main() {
    let matches = App::new("mutt-maildirs")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(Arg::with_name("base")
             .short("b")
             .long("base")
             .value_name("MAILDIR")
             .help("Base directory of the Maildir to sort")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name("initial")
             .short("i")
             .long("initial")
             .value_name("INITIAL")
             .help("Maildirs to be sorted first")
             .takes_value(true)
             .multiple(true))
        .arg(Arg::with_name("exclude")
             .short("e")
             .long("exclude")
             .value_name("EXCLUDE")
             .help("Maildirs to exclude from list")
             .takes_value(true)
             .multiple(true))
        .arg(Arg::with_name("verbose")
             .short("v")
             .long("verbose")
             .help("Set verbose mode"))
        .get_matches();

    let initial = match matches.values_of("initial") {
        None    => vec![],
        Some(x) => x.map(PathBuf::from).collect::<Vec<PathBuf>>(),
    };

    let excludes = match matches.values_of("exclude") {
        None    => vec![],
        Some(x) => x.map(PathBuf::from).collect::<Vec<PathBuf>>(),
    };

    // Unwrap here is safe since base is a required argument.
    let maildir_base = matches.value_of("base").unwrap();

    // Get mail directory list.
    let maildirs = list_maildirs(&maildir_base, &initial, &excludes);

    // Finally generate the output.
    // Iterate over the maildirs
    // .. wrap each one with the correct formatting for mutt.
    // .. collect the map output into a vector of strings
    // .. join that vector into a single string with entries seperated by space.
    let output = maildirs
        .iter()
        .map(|m| format!("+'{}'", m.display()))
        .collect::<Vec<String>>()
        .join(" ");

    println!("{}", output);
}
