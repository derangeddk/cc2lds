use std::{fs::{self, DirEntry}, fmt::Display, collections::HashMap};
use serde::ser::SerializeMap;
use yaml_rust::{YamlLoader, Yaml}; //TODO: replace yaml_rust with serde_yaml

pub enum DataHandlingError {
    IoError(std::io::Error),
    ScanError(yaml_rust::ScanError),
    ParsingError(ParsingError),
}

impl From<std::io::Error> for DataHandlingError {
    fn from(e: std::io::Error) -> Self {
        return DataHandlingError::IoError(e);
    }
}

impl From<yaml_rust::ScanError> for DataHandlingError {
    fn from(e: yaml_rust::ScanError) -> Self {
        return DataHandlingError::ScanError(e);
    }
}

impl Display for DataHandlingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            DataHandlingError::IoError(e) => write!(f, "YamlFileLoadingError:{}", e),
            DataHandlingError::ScanError(e) => write!(f, "YamlFileLoadingError:{}", e),
            DataHandlingError::ParsingError(e) => write!(f, "YamlFileLoadingError:{}", e),
        }
    }
}

pub struct ParsingError {
    pub error: String,
}

impl ParsingError {
    pub fn from_str(e: &str) -> Self {
        return Self { error: e.to_string() };
    }

    pub fn new(error: String) -> Self {
        return Self { error };
    }
}

impl Display for ParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "ParsingError:{}", self.error);
    }
}

impl From<ParsingError> for DataHandlingError {
    fn from(e: ParsingError) -> Self {
        return DataHandlingError::ParsingError(e);
    }
}

impl<T, U> NestedSimilar<Result<T, U>> for Result<Result<T, U>, U> {
    fn denest(self) -> Result<T, U> {
        return match self {
            Ok(res) => res,
            Err(e) => Err(e),
        };
    }
}

fn main() {
    let doc_load_result = load_yaml_docs("./data");
    let full_set_result = doc_load_result
        .map(|docs| merge_and_validate_docs(docs))
        .denest();

    match full_set_result {
        Ok(full_set) => output_lists(full_set).unwrap(), //TODO: get rid of unwrap
        Err(e) => eprintln!("Error while loading docs {}", e),
    }
}

#[derive(Clone)]
struct DomainTreeItem {
    value: String,
    children: Option<Vec<DomainTreeItem>>,
}

#[derive(Clone)]
struct DomainTreeRoot {
    children: Vec<DomainTreeItem>,
}

trait FlattenableTree<T> {
    fn flatten(self) -> Vec<T>;
}

impl FlattenableTree<String> for DomainTreeItem {
    fn flatten(self) -> Vec<String> {
        return flatten_domain_tree_item_with_prefix(self, "".to_string());
    }
}

impl FlattenableTree<String> for DomainTreeRoot {
    fn flatten(self) -> Vec<String> {
        let mut vec = Vec::new();
        for child in self.children {
            let mut recursed_child = child.flatten();
            vec.append(&mut recursed_child);
        }
        return vec;
    }
}

enum CompactTree {
    Node(HashMap<String, CompactTree>),
    Leaf(u8),
}

trait CompactableTree {
    fn compact(self) -> CompactTree;
}

impl CompactableTree for DomainTreeRoot {
    fn compact(self) -> CompactTree {
        return self.children.compact();
    }
}

impl CompactableTree for DomainTreeItem {
    fn compact(self) -> CompactTree {
        match self.children {
            Some(children) => children.compact(),
            None => CompactTree::Leaf(1),
        }
    }
}

impl CompactableTree for Vec<DomainTreeItem> {
    fn compact(self) -> CompactTree {
        let mut map = HashMap::new();

        for child in self {
            let key = child.value.clone();
            let value = child.compact();
            map.insert(key, value);
        }

        return CompactTree::Node(map);
    }
}

impl serde::ser::Serialize for CompactTree {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        match self {
            Self::Leaf(i) => serializer.serialize_u8(*i),
            Self::Node(n) => {
                let mut serialize_map = serializer.serialize_map(Some(n.len()))?;

                for (k, v) in n {
                    serialize_map.serialize_entry(k, v)?;
                }

                return serialize_map.end();
            },
        }
    }
}

fn flatten_domain_tree_item_with_prefix(item: DomainTreeItem, suffix: String) -> Vec<String> {
    let item_self: String = format!(".{}{}", item.value, suffix);
    let mut vec: Vec<String> = vec![item_self];

    match item.children {
        Some(children) => {
            let value = format!(".{}", item.value);

            for child in children {
                let mut recursed_child = flatten_domain_tree_item_with_prefix(child, format!("{}{}", value, suffix));
                vec.append(&mut recursed_child);
            }

            return vec;
        },
        None => vec,
    }
}

trait NestedSimilar<T> {
    fn denest(self) -> T;
}

//TODO: recursive parse_list definitely covers some of what happens here but more neatly
fn merge_and_validate_docs(docs: Vec<Yaml>) -> Result<DomainTreeRoot, DataHandlingError> {
    let mut full_set = Vec::new();

    for doc in docs {
        let hash = doc
            .as_hash()
            .ok_or(ParsingError::new(format!("Expected top-level doc to be a hash, for {:?}", doc)))?;
        
        for key in hash.keys() {
            let key_str = key
                .as_str()
                .ok_or(ParsingError::new(format!("Expected key {:?} to be a string in top-level doc {:?}", key, doc)))?
                .to_string();

            let yaml_list = hash[key]
                .as_vec()
                .ok_or(ParsingError::new(format!("Expected value {:?} for key {} to be a list in top-level doc {:?}", hash[key], key_str, doc)))?;
            
            let list = parse_domain_list(yaml_list, &key_str)?;

            // TODO: merge uniquely list if key_str already exists in full_set
            full_set.push(DomainTreeItem {
                value: key_str,
                children: Some(list),
            })
        }
    }

    return Ok(DomainTreeRoot { children: full_set });
}

fn parse_domain_list(yaml_list: &Vec<Yaml>, key_str: &String) -> Result<Vec<DomainTreeItem>, ParsingError> {
    let result = yaml_list
        .iter()
        .map(|item| match item {
            Yaml::String(s) => Ok(DomainTreeItem { value: s.to_string(), children: None }),
            Yaml::Hash(h) => {
                let value_opt = h
                    .get(&Yaml::String("value".to_string()))
                    .map(|val| val.as_str())
                    .flatten()
                    .map(|str| str.to_string());

                let children_opt = h
                    .get(&Yaml::String("children".to_string()))
                    .map(|val| val.as_vec())
                    .flatten();

                return match (value_opt, children_opt) {
                    (Some(value), Some(children)) => {
                        let parsed_children = parse_domain_list(children, key_str)?;
                        return Ok(DomainTreeItem { value, children: Some(parsed_children) });
                    },
                    _ => Err(ParsingError::new(format!("When parsing {} there was a Hash entry that did not have required `value` and `children` fields. Item: {:?}", key_str, item))),
                }
            },
            _ => Err(ParsingError::new(format!("Found invalid node when parsing {}. Node: {:?}", key_str, item))),
        })
        .collect::<Result<Vec<DomainTreeItem>, ParsingError>>()?;

    return Ok(result);
}

fn load_yaml_docs(dir: &str) -> Result<Vec<Yaml>, DataHandlingError> {
    let entries = fs::read_dir(dir)?
        .collect::<Result<Vec<DirEntry>, _>>()?;

    let yaml_file_paths = entries
        .iter()
        .map(|entry| entry.path())
        .filter(|path| {
            let maybe_str = path.to_str();
            if maybe_str.is_none() {
                return false;
            }
            let str = maybe_str.unwrap();
            return str.ends_with(".yml") || str.ends_with(".yaml");
        });

    // TODO: consider parallelizing file reads if this may give us a speed boost at large number of files?
    let yaml_file_string_contents = yaml_file_paths
        .map(|file_path| fs::read_to_string(file_path))
        .collect::<Result<Vec<String>, _>>()?;

    let nested_yaml_docs = yaml_file_string_contents
        .iter()
        .map(|string_content| YamlLoader::load_from_str(string_content))
        .collect::<Result<Vec<Vec<Yaml>>, _>>()?;
    
    let yaml_docs = nested_yaml_docs
        .iter()
        .flatten()
        .map(|doc| doc.to_owned())
        .collect();

    return Ok(yaml_docs);
}

fn output_lists(full_set: DomainTreeRoot) -> Result<(), std::io::Error> {
    // Output tree-like structure
    let compacted_tree = full_set.clone().compact();

    let json_tree = serde_json::to_string(&compacted_tree).unwrap(); // TODO: get rid og unwrap
    fs::write("output/2lds-tree.json", json_tree)?;

    let yml_tree = serde_yaml::to_string(&compacted_tree).unwrap(); // TODO: get rid og unwrap
    fs::write("output/2lds-tree.yml", yml_tree)?;

    // Output list-variant
    let fully_qualified_list = full_set.flatten();

    let json = serde_json::to_string(&fully_qualified_list).unwrap(); //TODO: et rid of unwrap
    fs::write("output/2lds.json", json)?;

    let yaml = serde_yaml::to_string(&fully_qualified_list).unwrap(); //TODO: get rid of unwrap
    fs::write("output/2lds.yml", yaml)?;

    return Ok(());
}
