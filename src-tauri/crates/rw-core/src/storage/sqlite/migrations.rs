use rusqlite::{Connection, Transaction};

use crate::{CoreError, CoreResult};

const MIGRATIONS: &[fn(&Transaction<'_>) -> rusqlite::Result<()>] =
    &[migration_1, migration_2, migration_3, migration_4];

pub(super) fn run(conn: &mut Connection) -> CoreResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY)",
    )
    .map_err(|err| CoreError::Storage(err.to_string()))?;

    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|err| CoreError::Storage(err.to_string()))?;

    for (offset, migration) in MIGRATIONS.iter().enumerate() {
        let version = (offset as i64) + 1;
        if version <= current {
            continue;
        }
        let tx = conn
            .transaction()
            .map_err(|err| CoreError::Storage(err.to_string()))?;
        migration(&tx)
            .map_err(|err| CoreError::Storage(format!("migration {version} failed: {err}")))?;
        tx.execute(
            "INSERT INTO schema_migrations (version) VALUES (?1)",
            [version],
        )
        .map_err(|err| CoreError::Storage(err.to_string()))?;
        tx.commit()
            .map_err(|err| CoreError::Storage(err.to_string()))?;
    }
    Ok(())
}

fn migration_1(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        r#"
        CREATE TABLE collections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            parent_id INTEGER REFERENCES collections(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE connections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            transport_kind TEXT NOT NULL CHECK (transport_kind IN ('foxglove_ws','rosbridge','native_ros2')),
            config_json TEXT NOT NULL,
            auto_connect INTEGER NOT NULL DEFAULT 0,
            color TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX idx_connections_name ON connections(name);

        CREATE TABLE requests (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            collection_id INTEGER REFERENCES collections(id) ON DELETE SET NULL,
            connection_id INTEGER REFERENCES connections(id) ON DELETE SET NULL,
            name TEXT NOT NULL,
            kind TEXT NOT NULL CHECK (kind IN ('topic','service','action')),
            target TEXT NOT NULL,
            schema_name TEXT,
            schema_hash TEXT,
            input_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX idx_requests_collection ON requests(collection_id);
        CREATE INDEX idx_requests_connection ON requests(connection_id);

        CREATE TABLE schemas (
            hash TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            definition TEXT NOT NULL,
            parsed_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE INDEX idx_schemas_name ON schemas(name);
        "#,
    )?;
    Ok(())
}

fn migration_2(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch("DELETE FROM schemas;")?;
    Ok(())
}

fn migration_3(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        r#"
        CREATE TABLE connections_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            transport_kind TEXT NOT NULL CHECK (transport_kind IN ('foxglove_ws','rosbridge','native_ros2','dummy')),
            config_json TEXT NOT NULL,
            auto_connect INTEGER NOT NULL DEFAULT 0,
            color TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        INSERT INTO connections_new SELECT * FROM connections;
        DROP TABLE connections;
        ALTER TABLE connections_new RENAME TO connections;
        CREATE INDEX idx_connections_name ON connections(name);
        "#,
    )?;
    Ok(())
}

fn migration_4(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch("ALTER TABLE requests ADD COLUMN visualization_json TEXT;")?;
    Ok(())
}
