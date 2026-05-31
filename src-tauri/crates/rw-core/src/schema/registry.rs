use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::domain::SchemaRef;
use crate::schema::{hash, parser, ParsedSchema, SchemaDefinition, SchemaKind};
use crate::storage::Storage;
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchemaSummary {
    pub name: String,
    pub hash: String,
    pub kind: SchemaKind,
    pub dependency_count: usize,
}

pub struct SchemaRegistry {
    storage: Arc<dyn Storage>,
    cache: RwLock<RegistryCache>,
}

#[derive(Default)]
struct RegistryCache {
    by_hash: BTreeMap<String, SchemaDefinition>,
    by_name: BTreeMap<String, Vec<String>>,
}

impl SchemaRegistry {
    pub async fn new(storage: Arc<dyn Storage>) -> CoreResult<Self> {
        let registry = Self {
            storage,
            cache: RwLock::new(RegistryCache::default()),
        };
        registry.refresh().await?;
        Ok(registry)
    }

    pub async fn refresh(&self) -> CoreResult<()> {
        let definitions = self.storage.list_schemas().await?;
        let mut cache = self.cache.write().expect("schema cache poisoned");
        cache.by_hash.clear();
        cache.by_name.clear();
        for definition in definitions {
            install_into_cache(&mut cache, definition);
        }
        Ok(())
    }

    pub async fn ensure_defaults(&self) -> CoreResult<()> {
        super::defaults::install_into(self).await
    }

    pub async fn register(
        &self,
        name: &str,
        kind: SchemaKind,
        definition: &str,
    ) -> CoreResult<SchemaRef> {
        let package = name.split('/').next().filter(|segment| !segment.is_empty());
        let parsed = parser::parse_with_package(kind, definition, package)?;
        let dependencies = parser::collect_dependencies(&parsed);

        let resolved: BTreeMap<String, String> = {
            let cache = self.cache.read().expect("schema cache poisoned");
            cache
                .by_name
                .iter()
                .filter_map(|(name, hashes)| {
                    let hash = hashes.first()?;
                    let entry = cache.by_hash.get(hash)?;
                    Some((name.clone(), entry.definition.clone()))
                })
                .collect()
        };

        let hash_value =
            hash::canonical_hash_with_package(kind, definition, package, |dep_name| {
                resolved.get(dep_name).map(String::as_str)
            })?;

        let dependency_set = collect_transitive_dependencies(&dependencies, &resolved)?;

        let entry = SchemaDefinition {
            name: name.to_string(),
            kind,
            hash: hash_value.clone(),
            definition: definition.to_string(),
            parsed,
            dependencies: dependency_set,
        };

        self.storage.put_schema(&entry).await?;
        let mut cache = self.cache.write().expect("schema cache poisoned");
        install_into_cache(&mut cache, entry);

        Ok(SchemaRef {
            name: name.to_string(),
            hash: hash_value,
        })
    }

    pub fn get_by_hash(&self, hash: &str) -> Option<SchemaDefinition> {
        let cache = self.cache.read().expect("schema cache poisoned");
        cache.by_hash.get(hash).cloned()
    }

    pub fn get_by_name(&self, name: &str) -> Vec<SchemaDefinition> {
        let cache = self.cache.read().expect("schema cache poisoned");
        cache
            .by_name
            .get(name)
            .map(|hashes| {
                hashes
                    .iter()
                    .filter_map(|hash| cache.by_hash.get(hash).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn list_names(&self) -> Vec<String> {
        let cache = self.cache.read().expect("schema cache poisoned");
        cache.by_name.keys().cloned().collect()
    }

    pub fn list_summaries(&self) -> Vec<SchemaSummary> {
        let cache = self.cache.read().expect("schema cache poisoned");
        cache
            .by_hash
            .values()
            .map(|definition| SchemaSummary {
                name: definition.name.clone(),
                hash: definition.hash.clone(),
                kind: definition.kind,
                dependency_count: definition.dependencies.len(),
            })
            .collect()
    }

    pub fn require_by_name(&self, name: &str) -> CoreResult<SchemaDefinition> {
        let candidates = self.get_by_name(name);
        candidates
            .into_iter()
            .next()
            .ok_or_else(|| CoreError::NotFound(format!("schema '{name}' is not registered")))
    }

    pub fn parsed(&self, hash: &str) -> Option<ParsedSchema> {
        self.get_by_hash(hash).map(|definition| definition.parsed)
    }
}

fn install_into_cache(cache: &mut RegistryCache, definition: SchemaDefinition) {
    let hash = definition.hash.clone();
    let name = definition.name.clone();
    cache.by_hash.insert(hash.clone(), definition);
    let entries = cache.by_name.entry(name).or_default();
    if !entries.contains(&hash) {
        entries.insert(0, hash);
    }
}

fn collect_transitive_dependencies(
    direct: &[String],
    resolver: &BTreeMap<String, String>,
) -> CoreResult<Vec<String>> {
    let mut visited: BTreeMap<String, ()> = BTreeMap::new();
    let mut stack: Vec<String> = direct.to_vec();
    while let Some(name) = stack.pop() {
        if visited.insert(name.clone(), ()).is_some() {
            continue;
        }
        if let Some(text) = resolver.get(&name) {
            let dep_package = name.split('/').next().filter(|segment| !segment.is_empty());
            let parsed = parser::parse_with_package(SchemaKind::Message, text, dep_package)?;
            for nested in parser::collect_dependencies(&parsed) {
                if !visited.contains_key(&nested) {
                    stack.push(nested);
                }
            }
        }
    }
    let mut ordered: Vec<String> = visited.into_keys().collect();
    ordered.sort();
    Ok(ordered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteStorage;
    use crate::util::MockClock;
    use chrono::TimeZone;

    async fn make_registry() -> SchemaRegistry {
        let clock = Arc::new(MockClock::new(
            chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        ));
        let storage: Arc<dyn Storage> =
            Arc::new(SqliteStorage::open_in_memory(clock).expect("in-memory storage"));
        SchemaRegistry::new(storage).await.unwrap()
    }

    #[tokio::test]
    async fn registers_and_round_trips() {
        let registry = make_registry().await;
        let reference = registry
            .register(
                "std_msgs/Header",
                SchemaKind::Message,
                "uint32 seq\nbuiltin_interfaces/Time stamp\nstring frame_id\n",
            )
            .await;
        assert!(matches!(reference, Err(CoreError::Schema(_))));

        registry
            .register(
                "builtin_interfaces/Time",
                SchemaKind::Message,
                "int32 sec\nuint32 nanosec\n",
            )
            .await
            .unwrap();
        let reference = registry
            .register(
                "std_msgs/Header",
                SchemaKind::Message,
                "uint32 seq\nbuiltin_interfaces/Time stamp\nstring frame_id\n",
            )
            .await
            .unwrap();
        assert_eq!(reference.name, "std_msgs/Header");
        assert_eq!(reference.hash.len(), 64);

        let by_name = registry.get_by_name("std_msgs/Header");
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].dependencies, vec!["builtin_interfaces/Time"]);
    }

    #[tokio::test]
    async fn duplicate_registration_is_idempotent() {
        let registry = make_registry().await;
        let reference_one = registry
            .register(
                "builtin_interfaces/Time",
                SchemaKind::Message,
                "int32 sec\nuint32 nanosec\n",
            )
            .await
            .unwrap();
        let reference_two = registry
            .register(
                "builtin_interfaces/Time",
                SchemaKind::Message,
                "int32 sec\nuint32 nanosec\n",
            )
            .await
            .unwrap();
        assert_eq!(reference_one.hash, reference_two.hash);
        assert_eq!(registry.get_by_name("builtin_interfaces/Time").len(), 1);
    }

    #[tokio::test]
    async fn different_definition_under_same_name_creates_new_version() {
        let registry = make_registry().await;
        registry
            .register("custom/Type", SchemaKind::Message, "int32 a\n")
            .await
            .unwrap();
        registry
            .register("custom/Type", SchemaKind::Message, "int32 a\nint32 b\n")
            .await
            .unwrap();
        let versions = registry.get_by_name("custom/Type");
        assert_eq!(versions.len(), 2);
    }
}
