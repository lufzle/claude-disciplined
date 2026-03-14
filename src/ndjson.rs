use crate::store::Store;

/// Load NDJSON records from a file, deserializing each line.
pub fn load<T, S: Store>(
    store: &S,
    path: &str,
    parse: fn(&str) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    let content = store.read_file(path).map_err(|e| e.to_string())?;
    content
        .lines()
        .filter(|line| !line.is_empty())
        .map(parse)
        .collect()
}

/// Save NDJSON records to a file, serializing each item to a line.
pub fn save<T, S: Store>(
    store: &S,
    path: &str,
    items: &[T],
    serialize: fn(&T) -> String,
) -> Result<(), String> {
    let content: String = items.iter().map(serialize).collect::<Vec<_>>().join("\n");
    let content = if content.is_empty() {
        String::new()
    } else {
        format!("{content}\n")
    };
    store.write_file(path, &content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemStore;

    #[allow(clippy::unnecessary_wraps)]
    fn parse_line(line: &str) -> Result<String, String> {
        Ok(line.to_owned())
    }

    #[allow(clippy::ptr_arg)]
    fn serialize_line(item: &String) -> String {
        item.clone()
    }

    #[test]
    fn load_empty_file() {
        let store = MemStore::new();
        store.write_file("test.ndjson", "").unwrap();
        let items: Vec<String> = load(&store, "test.ndjson", parse_line).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let store = MemStore::new();
        let items = vec!["hello".to_owned(), "world".to_owned()];
        save(&store, "test.ndjson", &items, serialize_line).unwrap();
        let loaded: Vec<String> = load(&store, "test.ndjson", parse_line).unwrap();
        assert_eq!(items, loaded);
    }

    #[test]
    fn save_empty_writes_empty() {
        let store = MemStore::new();
        let items: Vec<String> = vec![];
        save(&store, "test.ndjson", &items, serialize_line).unwrap();
        let content = store.read_file("test.ndjson").unwrap();
        assert!(content.is_empty());
    }
}
