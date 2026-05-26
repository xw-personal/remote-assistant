use rusqlite::{Connection, Result, params};
use std::sync::{Arc, Mutex};

pub type DbPool = Arc<Mutex<Connection>>;

pub fn init_db(path: &str) -> Result<DbPool> {
    let conn = Connection::open(path)?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS users (
            id          TEXT PRIMARY KEY,
            username    TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS devices (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            owner_id    TEXT NOT NULL,
            token       TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (owner_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS bindings (
            user_id     TEXT NOT NULL,
            device_id   TEXT NOT NULL,
            paired_at   TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (user_id, device_id),
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (device_id) REFERENCES devices(id)
        );

        CREATE TABLE IF NOT EXISTS pairing_codes (
            code        TEXT PRIMARY KEY,
            device_id   TEXT NOT NULL,
            expires_at  TEXT NOT NULL,
            used        INTEGER NOT NULL DEFAULT 0
        );"
    )?;

    Ok(Arc::new(Mutex::new(conn)))
}

pub fn create_user(conn: &DbPool, id: &str, username: &str, password_hash: &str) -> Result<()> {
    let c = conn.lock().unwrap();
    c.execute(
        "INSERT INTO users (id, username, password_hash) VALUES (?1, ?2, ?3)",
        params![id, username, password_hash],
    )?;
    Ok(())
}

pub fn get_user_by_username(conn: &DbPool, username: &str) -> Result<Option<(String, String, String)>> {
    let c = conn.lock().unwrap();
    let mut stmt = c.prepare("SELECT id, username, password_hash FROM users WHERE username = ?1")?;
    let mut rows = stmt.query(params![username])?;
    if let Some(row) = rows.next()? {
        Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?)))
    } else {
        Ok(None)
    }
}

pub fn register_device(conn: &DbPool, device_id: &str, name: &str, owner_id: &str, token: &str) -> Result<()> {
    let c = conn.lock().unwrap();
    c.execute(
        "INSERT INTO devices (id, name, owner_id, token) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET name = ?2, token = ?4",
        params![device_id, name, owner_id, token],
    )?;
    Ok(())
}

pub fn store_pairing_code(conn: &DbPool, code: &str, device_id: &str, expires_at: &str) -> Result<()> {
    let c = conn.lock().unwrap();
    c.execute(
        "INSERT INTO pairing_codes (code, device_id, expires_at) VALUES (?1, ?2, ?3)",
        params![code, device_id, expires_at],
    )?;
    Ok(())
}

pub fn verify_pairing_code(conn: &DbPool, code: &str) -> Result<Option<String>> {
    let c = conn.lock().unwrap();
    let now = chrono_now();
    let mut stmt = c.prepare(
        "SELECT device_id FROM pairing_codes WHERE code = ?1 AND used = 0 AND expires_at > ?2"
    )?;
    let mut rows = stmt.query(params![code, now])?;
    if let Some(row) = rows.next()? {
        let device_id: String = row.get(0)?;
        // Mark as used
        c.execute(
            "UPDATE pairing_codes SET used = 1 WHERE code = ?1",
            params![code],
        )?;
        Ok(Some(device_id))
    } else {
        Ok(None)
    }
}

pub fn bind_device(conn: &DbPool, user_id: &str, device_id: &str) -> Result<()> {
    let c = conn.lock().unwrap();
    c.execute(
        "INSERT OR IGNORE INTO bindings (user_id, device_id) VALUES (?1, ?2)",
        params![user_id, device_id],
    )?;
    Ok(())
}

pub fn get_user_devices(conn: &DbPool, user_id: &str) -> Result<Vec<(String, String, String)>> {
    let c = conn.lock().unwrap();
    let mut stmt = c.prepare(
        "SELECT d.id, d.name, b.paired_at
         FROM devices d
         JOIN bindings b ON d.id = b.device_id
         WHERE b.user_id = ?1"
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
    })?;
    let mut devices = Vec::new();
    for row in rows {
        devices.push(row?);
    }
    Ok(devices)
}

pub fn get_device_token(conn: &DbPool, device_id: &str) -> Result<Option<String>> {
    let c = conn.lock().unwrap();
    let mut stmt = c.prepare("SELECT token FROM devices WHERE id = ?1")?;
    let mut rows = stmt.query(params![device_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}
