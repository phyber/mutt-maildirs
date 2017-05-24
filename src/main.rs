/*
 * list-maildirs
 */
extern crate getopts;
extern crate walkdir;

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use getopts::Options;
use walkdir::{DirEntry, WalkDir, WalkDirIterator};

const TILDE: &'static str = "~";
const TILDE_SLASH: &'static str = "~/";

fn usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

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
        Some(path) => path,
        None => panic!("Could not get your home dir."),
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
// XXX: Feels like there should be a way to avoid OsStr::new here.
fn is_excluded(entry: &PathBuf, excluded: &Vec<String>) -> bool {
    excluded.iter().any(|x| OsStr::new(x) == entry)
}

// Checks if a maildir was listed as an initial maildir.
fn is_initial(maildir: &PathBuf, initial: &Vec<String>) -> bool {
    initial.iter().any(|x| Path::new(x) == maildir)
}

fn maildir_path(base: &Path, path: &Path) -> PathBuf {
    // Attempt to get the parent of the path we were given.
    // If we're successful, we strip the base prefix from it.
    let maildir = match path.parent() {
        Some(x) => x.strip_prefix(&base),
        None => panic!("No parent directory for {:?}", path),
    };

    // If stripping the base prefix was successful, return the maildir path.
    match maildir {
        Ok(m) => m.to_owned(),
        Err(e) => panic!("{}", e),
    }
}

fn list_maildirs(base: &str,
                 initial: &Vec<String>,
                 excluded: &Vec<String>)
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
        .filter_entry(|e| is_dir(e))
        .filter_map(Result::ok)
        .filter(|e| is_cur(e))
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
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optopt("b", "base", "set base directory", "BASEDIR");
    opts.optmulti("e", "exclude", "exclude maildir", "EXCLUDE");
    opts.optmulti("i", "initial", "initial maildirs", "INITIAL");
    opts.optflag("v", "verbose", "verbose mode");
    opts.optflag("h", "help", "display help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    // Print help if required.
    if matches.opt_present("h") {
        usage(&args[0], opts);
        return;
    }

    let initial = matches.opt_strs("i");
    let excludes = matches.opt_strs("e");

    // Get mail directory list.
    let maildirs = match matches.opt_str("b") {
        Some(x) => list_maildirs(&x, &initial, &excludes),
        None => panic!("Supply a basedir"),
    };

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
