// 会话历史层（M6.1 W2）
//
// 目标：记录跨会话历史、工具调用链与基础 trace 信息。
// 存储：SQLite + FTS5（<repo>/.aw/sessions/state.db）
//
// 表结构：
//   - sessions: 会话元信息
//   - turns: 单轮对话
//   - tool_traces: 工具调用记录
//   - sessions_fts: 会话全文检索虚拟表

use crate::error::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 工具调用 trace
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ToolCallTrace {
    /// 工具名
    pub tool_name: String,
    /// 调用状态（success / failed / timeout）
    pub status: String,
    /// 调用耗时（毫秒）
    #[serde(default)]
    pub duration_ms: u64,
}

impl ToolCallTrace {
    /// 创建 trace
    pub fn new(tool_name: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            status: status.into(),
            duration_ms: 0,
        }
    }

    /// 标记成功
    pub fn success(tool_name: impl Into<String>) -> Self {
        Self::new(tool_name, "success")
    }

    /// 标记失败
    pub fn failed(tool_name: impl Into<String>) -> Self {
        Self::new(tool_name, "failed")
    }

    /// 设置耗时
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// 是否成功
    pub fn is_success(&self) -> bool {
        self.status == "success"
    }
}

/// 单轮对话记录
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TurnRecord {
    /// 角色（user / assistant / system / tool）
    pub role: String,
    /// 文本内容
    pub content: String,
    /// 本轮 token 消耗
    #[serde(default)]
    pub tokens: u64,
    /// 本轮工具调用
    #[serde(default)]
    pub tool_calls: Vec<ToolCallTrace>,
}

impl TurnRecord {
    /// 创建一轮对话
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tokens: 0,
            tool_calls: Vec::new(),
        }
    }

    /// 设置 token
    pub fn with_tokens(mut self, tokens: u64) -> Self {
        self.tokens = tokens;
        self
    }

    /// 追加工具调用
    pub fn with_tool_call(mut self, trace: ToolCallTrace) -> Self {
        self.tool_calls.push(trace);
        self
    }

    /// 本轮所有工具调用次数
    pub fn tool_call_count(&self) -> usize {
        self.tool_calls.len()
    }

    /// 本轮成功的工具调用次数
    pub fn success_tool_count(&self) -> usize {
        self.tool_calls.iter().filter(|t| t.is_success()).count()
    }
}

/// 会话记录
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionRecord {
    /// 会话 ID
    pub session_id: String,
    /// 标题
    #[serde(default)]
    pub title: String,
    /// 关联员工 ID
    #[serde(default)]
    pub employee_id: String,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 轮次列表
    #[serde(default)]
    pub turns: Vec<TurnRecord>,
    /// token 消耗（聚合值）
    #[serde(default)]
    pub total_tokens: u64,
}

impl SessionRecord {
    /// 创建空会话
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            ..Self::default()
        }
    }

    /// 关联员工
    pub fn with_employee(mut self, employee_id: impl Into<String>) -> Self {
        self.employee_id = employee_id.into();
        self
    }

    /// 设置标题
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// 追加标签
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// 添加一轮对话（同时累加 token）
    pub fn push_turn(&mut self, turn: TurnRecord) {
        self.total_tokens += turn.tokens;
        self.turns.push(turn);
    }

    /// 返回轮次数
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// 全部工具调用次数
    pub fn total_tool_calls(&self) -> usize {
        self.turns.iter().map(|t| t.tool_calls.len()).sum()
    }

    /// 成功工具调用次数
    pub fn successful_tool_calls(&self) -> usize {
        self.turns.iter().map(|t| t.success_tool_count()).sum()
    }

    /// 关键词搜索（匹配 content）
    pub fn contains(&self, query: &str) -> bool {
        let q = query.to_ascii_lowercase();
        if q.is_empty() {
            return false;
        }
        self.turns
            .iter()
            .any(|t| t.content.to_ascii_lowercase().contains(&q))
    }

    /// 满意度评分（基于成功工具调用比例 + 无 failed turn 的简化启发式）
    pub fn satisfaction_hint(&self) -> f64 {
        let total = self.total_tool_calls();
        if total == 0 {
            return 1.0;
        }
        let success = self.successful_tool_calls();
        success as f64 / total as f64
    }

    /// 序列化为 JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 从 JSON 加载
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// 保存到文件
    pub fn save_file(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }

    /// 从文件加载
    pub fn load_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
    }
}

/// 会话存储（SQLite + FTS5）
pub struct SessionStore {
    conn: Connection,
}

impl SessionStore {
    /// 打开或创建数据库
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path.as_ref())?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// 在内存中创建（测试用）
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// 初始化表结构
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT '',
                employee_id TEXT NOT NULL DEFAULT '',
                tags TEXT NOT NULL DEFAULT '[]',
                total_tokens INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS turns (
                turn_id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                tokens INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS tool_traces (
                trace_id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                turn_id INTEGER NOT NULL,
                tool_name TEXT NOT NULL,
                status TEXT NOT NULL,
                duration_ms INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE,
                FOREIGN KEY (turn_id) REFERENCES turns(turn_id) ON DELETE CASCADE
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS sessions_fts USING fts5(
                session_id UNINDEXED,
                title,
                content,
                tokenize = 'unicode61'
            );

            CREATE TRIGGER IF NOT EXISTS sessions_after_insert
            AFTER INSERT ON sessions
            BEGIN
                INSERT INTO sessions_fts(session_id, title, content)
                VALUES (NEW.session_id, NEW.title, '');
            END;

            CREATE TRIGGER IF NOT EXISTS sessions_after_update
            AFTER UPDATE ON sessions
            BEGIN
                UPDATE sessions_fts SET title = NEW.title
                WHERE session_id = NEW.session_id;
            END;
            "#,
        )?;
        Ok(())
    }

    /// 保存一条会话（含 turns 和 tool_traces）
    pub fn save(&self, session: &SessionRecord) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO sessions (session_id, title, employee_id, tags, total_tokens)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(session_id) DO UPDATE SET
                title = excluded.title,
                employee_id = excluded.employee_id,
                tags = excluded.tags,
                total_tokens = excluded.total_tokens,
                updated_at = datetime('now')
            "#,
            params![
                session.session_id,
                session.title,
                session.employee_id,
                serde_json::to_string(&session.tags)?,
                session.total_tokens as i64,
            ],
        )?;

        // 删除旧的 turns 和 tool_traces，重新写入
        self.conn.execute(
            "DELETE FROM tool_traces WHERE session_id = ?1",
            params![session.session_id],
        )?;
        self.conn
            .execute("DELETE FROM turns WHERE session_id = ?1", params![session.session_id])?;

        let mut content_parts: Vec<String> = Vec::new();
        for turn in &session.turns {
            self.conn.execute(
                r#"
                INSERT INTO turns (session_id, role, content, tokens)
                VALUES (?1, ?2, ?3, ?4)
                "#,
                params![
                    session.session_id,
                    turn.role,
                    turn.content,
                    turn.tokens as i64,
                ],
            )?;
            let turn_id = self.conn.last_insert_rowid();
            content_parts.push(turn.content.clone());

            for trace in &turn.tool_calls {
                self.conn.execute(
                    r#"
                    INSERT INTO tool_traces (session_id, turn_id, tool_name, status, duration_ms)
                    VALUES (?1, ?2, ?3, ?4, ?5)
                    "#,
                    params![
                        session.session_id,
                        turn_id,
                        trace.tool_name,
                        trace.status,
                        trace.duration_ms as i64,
                    ],
                )?;
            }
        }

        // 更新全文检索内容（先删除旧记录，再重新插入，确保 content 始终同步）
        let content = content_parts.join("\n");
        self.conn.execute(
            "DELETE FROM sessions_fts WHERE session_id = ?1",
            params![session.session_id],
        )?;
        self.conn.execute(
            "INSERT INTO sessions_fts(session_id, title, content) VALUES (?1, ?2, ?3)",
            params![session.session_id, session.title, content],
        )?;

        Ok(())
    }

    /// 按 ID 加载完整会话
    pub fn load(&self, session_id: &str) -> Result<Option<SessionRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT session_id, title, employee_id, tags, total_tokens
            FROM sessions WHERE session_id = ?1
            "#,
        )?;
        let session_opt = stmt
            .query_row(params![session_id], |row| {
                Ok(SessionRecord {
                    session_id: row.get(0)?,
                    title: row.get(1)?,
                    employee_id: row.get(2)?,
                    tags: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                    total_tokens: row.get::<_, i64>(4)? as u64,
                    turns: Vec::new(),
                })
            })
            .optional()?;

        let Some(mut session) = session_opt else {
            return Ok(None);
        };

        // 加载 turns
        let mut turn_stmt = self.conn.prepare(
            r#"
            SELECT turn_id, role, content, tokens
            FROM turns WHERE session_id = ?1 ORDER BY turn_id
            "#,
        )?;
        let turn_rows = turn_stmt.query_map(params![session_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                TurnRecord {
                    role: row.get(1)?,
                    content: row.get(2)?,
                    tokens: row.get::<_, i64>(3)? as u64,
                    tool_calls: Vec::new(),
                },
            ))
        })?;

        let mut turns = Vec::new();
        for row in turn_rows {
            let (turn_id, mut turn) = row?;

            // 加载该 turn 的 tool_traces
            let mut trace_stmt = self.conn.prepare(
                r#"
                SELECT tool_name, status, duration_ms
                FROM tool_traces WHERE turn_id = ?1 ORDER BY trace_id
                "#,
            )?;
            let traces = trace_stmt.query_map(params![turn_id], |row| {
                Ok(ToolCallTrace {
                    tool_name: row.get(0)?,
                    status: row.get(1)?,
                    duration_ms: row.get::<_, i64>(2)? as u64,
                })
            })?;
            for trace in traces {
                turn.tool_calls.push(trace?);
            }
            turns.push(turn);
        }
        session.turns = turns;
        Ok(Some(session))
    }

    /// 全文检索会话（FTS5 优先，未命中时用 LIKE 兜底）
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SessionRecord>> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }

        // 1) 先尝试 FTS5
        let sql = format!(
            r#"
            SELECT s.session_id
            FROM sessions_fts fts
            JOIN sessions s ON fts.session_id = s.session_id
            WHERE sessions_fts MATCH ?1
            ORDER BY rank
            LIMIT {}
            "#,
            limit
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let ids: Vec<String> = stmt
            .query_map(params![q], |row| row.get(0))?
            .collect::<std::result::Result<_, _>>()?;

        if !ids.is_empty() {
            let mut results = Vec::new();
            for id in ids {
                if let Some(session) = self.load(&id)? {
                    results.push(session);
                }
            }
            return Ok(results);
        }

        // 2) FTS5 未命中（尤其中文分词不支持时），退化为 LIKE 查询
        let pattern = format!("%{}%", q);
        let mut stmt = self.conn.prepare(
            r#"
            SELECT DISTINCT s.session_id
            FROM sessions s
            LEFT JOIN turns t ON t.session_id = s.session_id
            WHERE s.title LIKE ?1
               OR s.tags LIKE ?1
               OR t.content LIKE ?1
            ORDER BY s.updated_at DESC
            LIMIT ?2
            "#,
        )?;
        let ids: Vec<String> = stmt
            .query_map(params![pattern, limit as i64], |row| row.get(0))?
            .collect::<std::result::Result<_, _>>()?;

        let mut results = Vec::new();
        for id in ids {
            if let Some(session) = self.load(&id)? {
                results.push(session);
            }
        }
        Ok(results)
    }

    /// 按员工 ID 筛选会话
    pub fn by_employee(&self, employee_id: &str, limit: usize) -> Result<Vec<SessionRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT session_id FROM sessions
            WHERE employee_id = ?1
            ORDER BY updated_at DESC
            LIMIT ?2
            "#,
        )?;
        let ids: Vec<String> = stmt
            .query_map(params![employee_id, limit as i64], |row| row.get(0))?
            .collect::<std::result::Result<_, _>>()?;

        let mut results = Vec::new();
        for id in ids {
            if let Some(session) = self.load(&id)? {
                results.push(session);
            }
        }
        Ok(results)
    }

    /// 列出最近会话
    pub fn list_recent(&self, limit: usize) -> Result<Vec<SessionRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT session_id FROM sessions
            ORDER BY updated_at DESC
            LIMIT ?1
            "#,
        )?;
        let ids: Vec<String> = stmt
            .query_map(params![limit as i64], |row| row.get(0))?
            .collect::<std::result::Result<_, _>>()?;

        let mut results = Vec::new();
        for id in ids {
            if let Some(session) = self.load(&id)? {
                results.push(session);
            }
        }
        Ok(results)
    }

    /// 删除会话
    pub fn delete(&self, session_id: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "DELETE FROM sessions WHERE session_id = ?1",
            params![session_id],
        )?;
        Ok(rows > 0)
    }

    /// 统计总 token 消耗
    pub fn total_tokens(&self) -> Result<u64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COALESCE(SUM(total_tokens), 0) FROM sessions", [], |row| {
                row.get(0)
            })?;
        Ok(count as u64)
    }

    /// 统计总会话数
    pub fn session_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// 统计总工具调用次数
    pub fn total_tool_calls(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM tool_traces", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// 导出会话为 JSON（兼容旧格式，便于分享）
    pub fn export_json(&self, session_id: &str) -> Result<Option<String>> {
        let session = self.load(session_id)?;
        match session {
            Some(s) => Ok(Some(s.to_json()?)),
            None => Ok(None),
        }
    }

    /// 从 JSON 导入会话
    pub fn import_json(&self, json: &str) -> Result<String> {
        let session = SessionRecord::from_json(json)?;
        self.save(&session)?;
        Ok(session.session_id)
    }

    /// 默认数据库存储路径
    pub fn default_db_path(repo_root: &Path) -> PathBuf {
        repo_root.join(".aw").join("sessions").join("state.db")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str) -> SessionRecord {
        SessionRecord::new(id)
            .with_title("测试会话")
            .with_employee("emp-1")
    }

    #[test]
    fn sqlite_save_and_load() {
        let store = SessionStore::open_in_memory().unwrap();
        let mut s = session("s1");
        s.push_turn(TurnRecord::new("user", "退款").with_tokens(10));
        s.push_turn(
            TurnRecord::new("assistant", "好的")
                .with_tokens(20)
                .with_tool_call(ToolCallTrace::success("query_order")),
        );
        store.save(&s).unwrap();

        let loaded = store.load("s1").unwrap().unwrap();
        assert_eq!(loaded.session_id, "s1");
        assert_eq!(loaded.turn_count(), 2);
        assert_eq!(loaded.total_tokens, 30);
        assert_eq!(loaded.total_tool_calls(), 1);
        assert_eq!(loaded.successful_tool_calls(), 1);
    }

    #[test]
    fn sqlite_search_fts() {
        let store = SessionStore::open_in_memory().unwrap();
        let mut s1 = session("s1").with_title("refund consultation");
        s1.push_turn(TurnRecord::new("user", "I want a refund for the PDF error").with_tokens(10));
        store.save(&s1).unwrap();

        let mut s2 = session("s2").with_title("complaint").with_employee("emp-2");
        s2.push_turn(TurnRecord::new("user", "The service attitude is bad").with_tokens(10));
        store.save(&s2).unwrap();

        // 英文关键词（验证 FTS5 基础链路）
        let results = store.search("refund", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "s1");

        let results = store.search("service", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "s2");

        // 中文场景测试：验证 LIKE 兜底可用
        let mut s3 = session("s3").with_title("中文测试").with_employee("emp-3");
        s3.push_turn(TurnRecord::new("user", "我想退款怎么办").with_tokens(10));
        store.save(&s3).unwrap();
        let results = store.search("退款", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "s3");
    }

    #[test]
    fn sqlite_by_employee_and_aggregates() {
        let store = SessionStore::open_in_memory().unwrap();
        let mut s1 = session("s1").with_tag("refund");
        s1.push_turn(TurnRecord::new("user", "退款流程").with_tokens(10));
        let mut s2 = session("s2").with_tag("complaint").with_employee("emp-2");
        s2.push_turn(
            TurnRecord::new("assistant", "投诉处理")
                .with_tokens(20)
                .with_tool_call(ToolCallTrace::success("escalate")),
        );
        store.save(&s1).unwrap();
        store.save(&s2).unwrap();

        assert_eq!(store.session_count().unwrap(), 2);
        assert_eq!(store.total_tokens().unwrap(), 30);
        assert_eq!(store.total_tool_calls().unwrap(), 1);
        assert_eq!(store.by_employee("emp-1", 10).unwrap().len(), 1);
        assert_eq!(store.by_employee("emp-2", 10).unwrap().len(), 1);
    }

    #[test]
    fn json_roundtrip() {
        let mut s = session("s1");
        s.push_turn(
            TurnRecord::new("user", "x")
                .with_tokens(5)
                .with_tool_call(ToolCallTrace::success("t")),
        );
        let json = s.to_json().unwrap();
        let restored = SessionRecord::from_json(&json).unwrap();
        assert_eq!(restored.session_id, "s1");
        assert_eq!(restored.total_tokens, 5);
        assert_eq!(restored.turns[0].tool_calls.len(), 1);
    }

    #[test]
    fn file_roundtrip() {
        let path = std::env::temp_dir().join("aw_session_test.json");
        let mut s = session("s-file");
        s.push_turn(TurnRecord::new("user", "hello").with_tokens(3));
        s.save_file(&path).unwrap();
        let loaded = SessionRecord::load_file(&path).unwrap();
        assert_eq!(loaded.session_id, "s-file");
        assert_eq!(loaded.total_tokens, 3);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn satisfaction_hint_with_failures() {
        let mut s = session("s1");
        s.push_turn(
            TurnRecord::new("assistant", "处理")
                .with_tool_call(ToolCallTrace::success("a"))
                .with_tool_call(ToolCallTrace::failed("b")),
        );
        let hint = s.satisfaction_hint();
        assert!((hint - 0.5).abs() < 1e-6);
    }
}
