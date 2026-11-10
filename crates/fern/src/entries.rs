use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub dir: bool,
}

pub fn read(path: &Path, hidden: bool) -> Vec<Entry> {
    let Ok(listing) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = listing
        .filter_map(Result::ok)
        .filter_map(|item| {
            let name = item.file_name().to_string_lossy().into_owned();
            if name == ".git" || (!hidden && name.starts_with('.')) {
                return None;
            }

            Some(Entry {
                name,
                dir: item.path().is_dir(),
            })
        })
        .collect();
    entries.sort_by_cached_key(|entry| (!entry.dir, entry.name.to_lowercase()));

    entries
}
