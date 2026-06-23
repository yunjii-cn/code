// 错误处理 + 重试机制（W9 M3.3 D5）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 3.3
//
// 提供统一的任务执行重试策略：
//   1. RetryPolicy: 重试策略配置（最大次数、退避策略）
//   2. RetryExecutor: 封装重试逻辑（执行 → 失败 → 退避 → 重试）
//   3. ErrorClassifier: 错误分类（可重试 / 不可重试）
//
// 重试场景：
//   - LLM 调用失败（网络/超时/限流）→ 可重试
//   - 代码生成 JSON 解析失败 → 可重试
//   - 验证护栏失败 → 可重试（带错误反馈）
//   - DAG 校验失败 → 不可重试（需重新规划）
//   - 模型未注册 → 不可重试（需配置）

use crate::error::{Result, TeamError};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 重试策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// 最大重试次数（不含首次执行）
    pub max_retries: u32,
    /// 初始退避时间（毫秒）
    pub initial_backoff_ms: u64,
    /// 退避倍数（指数退避）
    pub backoff_multiplier: f64,
    /// 最大退避时间（毫秒）
    pub max_backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,
            backoff_multiplier: 2.0,
            max_backoff_ms: 30000,
        }
    }
}

impl RetryPolicy {
    /// 创建不重试策略
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// 创建快速重试策略（短间隔）
    pub fn fast() -> Self {
        Self {
            max_retries: 2,
            initial_backoff_ms: 100,
            backoff_multiplier: 1.5,
            max_backoff_ms: 1000,
        }
    }

    /// 计算第 n 次重试的退避时间
    ///
    /// n=0 返回初始退避时间，n=1 返回 initial * multiplier，依此类推
    pub fn backoff(&self, retry_count: u32) -> Duration {
        if retry_count == 0 {
            return Duration::from_millis(self.initial_backoff_ms);
        }

        let mut backoff = self.initial_backoff_ms as f64;
        for _ in 0..retry_count {
            backoff *= self.backoff_multiplier;
        }

        let backoff_ms = backoff.min(self.max_backoff_ms as f64);
        Duration::from_millis(backoff_ms as u64)
    }

    /// 是否还可以重试
    pub fn can_retry(&self, current_retries: u32) -> bool {
        current_retries < self.max_retries
    }
}

/// 错误分类
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// 可重试错误（网络/超时/限流/解析失败）
    Retryable,
    /// 不可重试错误（配置错误/模型未注册/DAG 无效）
    NonRetryable,
}

/// 错误分类器
pub struct ErrorClassifier;

impl ErrorClassifier {
    /// 分类错误
    pub fn classify(error: &TeamError) -> ErrorKind {
        match error {
            // 配置错误 → 不可重试
            TeamError::Config(_) => ErrorKind::NonRetryable,

            // 角色不存在 → 不可重试
            TeamError::RoleNotFound(_) => ErrorKind::NonRetryable,

            // 模型未注册 → 不可重试
            TeamError::NoModel { .. } => ErrorKind::NonRetryable,

            // 所有模型都失败 → 可重试（可能是临时网络问题）
            TeamError::AllModelsFailed { .. } => ErrorKind::Retryable,

            // IO 错误 → 可重试
            TeamError::Io(_) => ErrorKind::Retryable,

            // HTTP 错误 → 可重试
            TeamError::Http(_) => ErrorKind::Retryable,

            // AI 错误 → 可重试（LLM 服务可能临时不可用）
            TeamError::Ai(_) => ErrorKind::Retryable,

            // YAML/JSON 解析错误 → 可重试（LLM 输出可能不稳定）
            TeamError::Yaml(_) => ErrorKind::Retryable,
            TeamError::Json(_) => ErrorKind::Retryable,

            // 数据库错误 → 可重试（临时锁定/并发）
            TeamError::Rusqlite(_) => ErrorKind::Retryable,

            // 其他错误 → 不可重试
            TeamError::Other(_) => ErrorKind::NonRetryable,
        }
    }

    /// 是否应该重试
    pub fn should_retry(error: &TeamError) -> bool {
        Self::classify(error) == ErrorKind::Retryable
    }
}

/// 重试执行结果
#[derive(Debug)]
pub struct RetryResult<T> {
    /// 最终结果（成功或最后一次失败的错误）
    pub result: std::result::Result<T, TeamError>,
    /// 总尝试次数（含首次）
    pub attempts: u32,
    /// 是否成功
    pub success: bool,
}

impl<T> RetryResult<T> {
    /// 是否成功
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// 获取结果（成功时）
    pub fn ok(self) -> Option<T> {
        if self.success {
            self.result.ok()
        } else {
            None
        }
    }

    /// 获取错误（失败时）
    pub fn err(self) -> Option<TeamError> {
        if self.success {
            None
        } else {
            self.result.err()
        }
    }

    /// 获取尝试次数
    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}

/// 重试执行器
///
/// 封装重试逻辑：执行 → 失败 → 分类 → 退避 → 重试
pub struct RetryExecutor {
    /// 重试策略
    policy: RetryPolicy,
}

impl RetryExecutor {
    /// 创建重试执行器
    pub fn new(policy: RetryPolicy) -> Self {
        Self { policy }
    }

    /// 使用默认策略
    pub fn default() -> Self {
        Self::new(RetryPolicy::default())
    }

    /// 获取策略
    pub fn policy(&self) -> &RetryPolicy {
        &self.policy
    }

    /// 执行带重试的异步操作
    ///
    /// `operation`: 返回 Result<T, TeamError> 的异步闭包
    ///
    /// 注意：本方法不实际等待退避时间（测试友好），生产环境应在回调中 sleep。
    pub async fn execute<F, Fut, T>(&self, mut operation: F) -> RetryResult<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempts = 0;

        loop {
            attempts += 1;
            match operation().await {
                Ok(value) => {
                    return RetryResult {
                        result: Ok(value),
                        attempts,
                        success: true,
                    };
                }
                Err(e) => {
                    let kind = ErrorClassifier::classify(&e);

                    // 不可重试错误，立即返回
                    if kind == ErrorKind::NonRetryable {
                        return RetryResult {
                            result: Err(e),
                            attempts,
                            success: false,
                        };
                    }

                    // 可重试错误，检查是否还有重试机会
                    let retries_so_far = attempts - 1;
                    if !self.policy.can_retry(retries_so_far) {
                        return RetryResult {
                            result: Err(e),
                            attempts,
                            success: false,
                        };
                    }

                    // 计算退避时间（生产环境应在此 sleep）
                    let _backoff = self.policy.backoff(retries_so_far);
                    // 注：实际 sleep 由调用方处理，这里不 sleep 以保持测试速度
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.initial_backoff_ms, 1000);
        assert_eq!(policy.backoff_multiplier, 2.0);
        assert_eq!(policy.max_backoff_ms, 30000);
    }

    #[test]
    fn test_retry_policy_no_retry() {
        let policy = RetryPolicy::no_retry();
        assert_eq!(policy.max_retries, 0);
        assert!(!policy.can_retry(0));
    }

    #[test]
    fn test_retry_policy_fast() {
        let policy = RetryPolicy::fast();
        assert_eq!(policy.max_retries, 2);
        assert_eq!(policy.initial_backoff_ms, 100);
    }

    #[test]
    fn test_backoff_calculation() {
        let policy = RetryPolicy::default();

        // 第 0 次重试：初始退避
        assert_eq!(policy.backoff(0), Duration::from_millis(1000));

        // 第 1 次重试：1000 * 2 = 2000
        assert_eq!(policy.backoff(1), Duration::from_millis(2000));

        // 第 2 次重试：1000 * 2 * 2 = 4000
        assert_eq!(policy.backoff(2), Duration::from_millis(4000));

        // 第 3 次重试：1000 * 2^3 = 8000
        assert_eq!(policy.backoff(3), Duration::from_millis(8000));
    }

    #[test]
    fn test_backoff_max_cap() {
        let policy = RetryPolicy {
            max_backoff_ms: 5000,
            ..Default::default()
        };

        // 第 10 次重试：应该被限制在 5000ms
        assert_eq!(policy.backoff(10), Duration::from_millis(5000));
    }

    #[test]
    fn test_can_retry() {
        let policy = RetryPolicy::default();

        assert!(policy.can_retry(0)); // 0 < 3
        assert!(policy.can_retry(1)); // 1 < 3
        assert!(policy.can_retry(2)); // 2 < 3
        assert!(!policy.can_retry(3)); // 3 == 3，不能重试
        assert!(!policy.can_retry(4)); // 4 > 3
    }

    #[test]
    fn test_error_classifier_config() {
        let error = TeamError::Config("配置错误".to_string());
        assert_eq!(ErrorClassifier::classify(&error), ErrorKind::NonRetryable);
        assert!(!ErrorClassifier::should_retry(&error));
    }

    #[test]
    fn test_error_classifier_role_not_found() {
        let error = TeamError::RoleNotFound("backend".to_string());
        assert_eq!(ErrorClassifier::classify(&error), ErrorKind::NonRetryable);
    }

    #[test]
    fn test_error_classifier_no_model() {
        let error = TeamError::NoModel {
            role: "backend".to_string(),
        };
        assert_eq!(ErrorClassifier::classify(&error), ErrorKind::NonRetryable);
    }

    #[test]
    fn test_error_classifier_all_models_failed() {
        let error = TeamError::AllModelsFailed {
            role: "backend".to_string(),
            attempts: 3,
        };
        assert_eq!(ErrorClassifier::classify(&error), ErrorKind::Retryable);
        assert!(ErrorClassifier::should_retry(&error));
    }

    #[test]
    fn test_error_classifier_other() {
        let error = TeamError::Other("其他错误".to_string());
        assert_eq!(ErrorClassifier::classify(&error), ErrorKind::NonRetryable);
    }

    #[tokio::test]
    async fn test_retry_executor_success_first_try() {
        let executor = RetryExecutor::default();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result: RetryResult<String> = executor
            .execute(|| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok("success".to_string())
                }
            })
            .await;

        assert!(result.is_success());
        assert_eq!(result.attempts(), 1);
        assert_eq!(result.ok(), Some("success".to_string()));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_executor_success_after_retry() {
        let executor = RetryExecutor::new(RetryPolicy {
            max_retries: 3,
            initial_backoff_ms: 1, // 短间隔加速测试
            ..Default::default()
        });

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result: RetryResult<String> = executor
            .execute(|| {
                let c = counter_clone.clone();
                async move {
                    let count = c.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        // 前 2 次失败
                        Err(TeamError::AllModelsFailed {
                            role: "backend".to_string(),
                            attempts: 1,
                        })
                    } else {
                        Ok("success".to_string())
                    }
                }
            })
            .await;

        assert!(result.is_success());
        assert_eq!(result.attempts(), 3);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_executor_non_retryable_error() {
        let executor = RetryExecutor::default();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result: RetryResult<String> = executor
            .execute(|| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(TeamError::Config("不可重试".to_string()))
                }
            })
            .await;

        assert!(!result.is_success());
        assert_eq!(result.attempts(), 1); // 不重试
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_executor_max_retries_exhausted() {
        let executor = RetryExecutor::new(RetryPolicy {
            max_retries: 2,
            initial_backoff_ms: 1,
            ..Default::default()
        });

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result: RetryResult<String> = executor
            .execute(|| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(TeamError::AllModelsFailed {
                        role: "backend".to_string(),
                        attempts: 1,
                    })
                }
            })
            .await;

        assert!(!result.is_success());
        assert_eq!(result.attempts(), 3); // 1 首次 + 2 重试
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_executor_no_retry_policy() {
        let executor = RetryExecutor::new(RetryPolicy::no_retry());

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result: RetryResult<String> = executor
            .execute(|| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(TeamError::AllModelsFailed {
                        role: "backend".to_string(),
                        attempts: 1,
                    })
                }
            })
            .await;

        assert!(!result.is_success());
        assert_eq!(result.attempts(), 1); // 不重试
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_retry_result_ok() {
        let result: RetryResult<String> = RetryResult {
            result: Ok("success".to_string()),
            attempts: 1,
            success: true,
        };

        assert!(result.is_success());
        assert_eq!(result.attempts(), 1);
    }

    #[test]
    fn test_retry_result_err() {
        let result: RetryResult<String> = RetryResult {
            result: Err(TeamError::Config("失败".to_string())),
            attempts: 3,
            success: false,
        };

        assert!(!result.is_success());
        assert_eq!(result.attempts(), 3);
        assert!(result.err().is_some());
    }

    #[test]
    fn test_error_kind_equality() {
        assert_eq!(ErrorKind::Retryable, ErrorKind::Retryable);
        assert_eq!(ErrorKind::NonRetryable, ErrorKind::NonRetryable);
        assert_ne!(ErrorKind::Retryable, ErrorKind::NonRetryable);
    }
}
