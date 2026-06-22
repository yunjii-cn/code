// 社区模板市场机制（M4.2 D4）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.2 D4
//
// 设计理念：把"我一个人做 5 个"变成"社区做 50 个"
//   - TemplateMarketEntry: 市场条目（公开模板的元信息 + 评分 + 评论）
//   - TemplateSubmission: 创作者提交（含 YAML + 文档 + 校验结果）
//   - TemplateReview: 审核流程（自动 LLM 评估 + 人工审核）
//   - TemplateRating: 用户评分（5 星 + 评论）
//   - TemplateMarketplace: 市场聚合（浏览/搜索/安装/提交/审核）
//
// 验收标准：
//   - 客户可在市场浏览 / 安装社区模板
//   - 创作者可提交模板赚钱
//   - 平台可审核质量

use crate::error::{Result, TeamError};
use crate::template_loader::TemplateManifest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// 市场 ID
pub type MarketId = String;

/// 作者 ID
pub type AuthorId = String;

/// 模板定价类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PricingType {
    /// 免费
    Free,
    /// 付费
    Paid,
}

impl Default for PricingType {
    fn default() -> Self {
        Self::Free
    }
}

impl std::fmt::Display for PricingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Free => write!(f, "free"),
            Self::Paid => write!(f, "paid"),
        }
    }
}

/// 模板审核状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReviewStatus {
    /// 待提交（草稿）
    Draft,
    /// 已提交（待审核）
    Submitted,
    /// 自动评估中
    AutoReviewing,
    /// 待人工审核
    PendingManual,
    /// 已通过
    Approved,
    /// 已拒绝
    Rejected,
    /// 已下架
    Delisted,
}

impl Default for ReviewStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl std::fmt::Display for ReviewStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Submitted => write!(f, "submitted"),
            Self::AutoReviewing => write!(f, "auto_reviewing"),
            Self::PendingManual => write!(f, "pending_manual"),
            Self::Approved => write!(f, "approved"),
            Self::Rejected => write!(f, "rejected"),
            Self::Delisted => write!(f, "delisted"),
        }
    }
}

/// 模板评分维度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRating {
    /// 评分者 ID
    pub user_id: String,
    /// 评分（1-5 星）
    pub stars: u8,
    /// 评论内容
    #[serde(default)]
    pub comment: String,
    /// 评分时间
    pub rated_at: DateTime<Utc>,
}

impl TemplateRating {
    /// 创建评分
    pub fn new(user_id: impl Into<String>, stars: u8, comment: impl Into<String>) -> Result<Self> {
        if stars < 1 || stars > 5 {
            return Err(TeamError::Other("评分必须在 1-5 星之间".into()));
        }
        Ok(Self {
            user_id: user_id.into(),
            stars,
            comment: comment.into(),
            rated_at: Utc::now(),
        })
    }
}

/// 模板评分聚合
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RatingSummary {
    /// 总评分人数
    pub count: usize,
    /// 平均分（0.0 - 5.0）
    pub average: f32,
    /// 各星级人数
    pub stars_count: [usize; 5],
}

impl RatingSummary {
    /// 从评分列表计算聚合
    pub fn from_ratings(ratings: &[TemplateRating]) -> Self {
        let mut summary = Self::default();
        summary.count = ratings.len();
        if ratings.is_empty() {
            return summary;
        }
        let mut total: u32 = 0;
        for r in ratings {
            let idx = (r.stars as usize).saturating_sub(1).min(4);
            summary.stars_count[idx] += 1;
            total += r.stars as u32;
        }
        summary.average = total as f32 / summary.count as f32;
        summary
    }
}

/// 模板市场条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMarketEntry {
    /// 市场 ID（唯一）
    pub id: MarketId,
    /// 模板 ID（与 manifest.id 一致）
    pub template_id: String,
    /// 模板名称
    pub name: String,
    /// 模板版本
    pub version: String,
    /// 行业
    pub industry: String,
    /// 子行业
    #[serde(default)]
    pub sub_industry: String,
    /// 描述
    #[serde(default)]
    pub description: String,
    /// 作者 ID
    pub author_id: AuthorId,
    /// 作者名称
    #[serde(default)]
    pub author_name: String,
    /// 定价类型
    #[serde(default)]
    pub pricing: PricingType,
    /// 价格（分，Paid 时有效；Free 为 0）
    #[serde(default)]
    pub price_cents: u64,
    /// 审核状态
    #[serde(default)]
    pub review_status: ReviewStatus,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 员工数
    #[serde(default)]
    pub employee_count: usize,
    /// 下载量
    #[serde(default)]
    pub download_count: u64,
    /// 评分聚合
    #[serde(default)]
    pub rating: RatingSummary,
    /// 评分列表
    #[serde(default)]
    pub ratings: Vec<TemplateRating>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 下载 URL（公开模板的 ZIP 包）
    #[serde(default)]
    pub download_url: Option<String>,
    /// 预览图 URL
    #[serde(default)]
    pub preview_images: Vec<String>,
}

impl TemplateMarketEntry {
    /// 创建新市场条目
    pub fn new(
        template_id: impl Into<String>,
        name: impl Into<String>,
        author_id: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: generate_market_id(),
            template_id: template_id.into(),
            name: name.into(),
            version: "1.0.0".to_string(),
            industry: String::new(),
            sub_industry: String::new(),
            description: String::new(),
            author_id: author_id.into(),
            author_name: String::new(),
            pricing: PricingType::default(),
            price_cents: 0,
            review_status: ReviewStatus::default(),
            tags: vec![],
            employee_count: 0,
            download_count: 0,
            rating: RatingSummary::default(),
            ratings: vec![],
            created_at: now,
            updated_at: now,
            download_url: None,
            preview_images: vec![],
        }
    }

    /// 从 manifest 创建条目
    pub fn from_manifest(manifest: &TemplateManifest, author_id: impl Into<String>) -> Self {
        let mut entry = Self::new(manifest.id.clone(), manifest.name.clone(), author_id);
        entry.version = manifest.version.clone();
        entry.industry = manifest.industry.clone();
        entry.sub_industry = manifest.sub_industry.clone();
        entry.description = manifest.description.clone();
        entry.employee_count = manifest.employee_count();
        entry
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 添加标签
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        let t = tag.into();
        if !self.tags.contains(&t) {
            self.tags.push(t);
        }
        self
    }

    /// 设置定价
    pub fn with_pricing(mut self, pricing: PricingType, price_cents: u64) -> Self {
        self.pricing = pricing;
        self.price_cents = if pricing == PricingType::Free { 0 } else { price_cents };
        self
    }

    /// 设置下载 URL
    pub fn with_download_url(mut self, url: impl Into<String>) -> Self {
        self.download_url = Some(url.into());
        self
    }

    /// 添加评分
    pub fn add_rating(&mut self, rating: TemplateRating) {
        self.ratings.push(rating);
        self.rating = RatingSummary::from_ratings(&self.ratings);
        self.updated_at = Utc::now();
    }

    /// 增加下载量
    pub fn increment_downloads(&mut self) {
        self.download_count += 1;
        self.updated_at = Utc::now();
    }

    /// 是否通过审核
    pub fn is_approved(&self) -> bool {
        self.review_status == ReviewStatus::Approved
    }

    /// 是否可被搜索到（已通过审核 + 未下架）
    pub fn is_listed(&self) -> bool {
        self.review_status == ReviewStatus::Approved
    }

    /// 价格显示（元）
    pub fn price_yuan(&self) -> f64 {
        self.price_cents as f64 / 100.0
    }
}

/// 生成市场 ID
fn generate_market_id() -> MarketId {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let ts = Utc::now().timestamp_millis();
    format!("mkt_{ts}_{n:06}")
}

/// 模板提交（创作者上传）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSubmission {
    /// 提交 ID
    pub id: String,
    /// 模板清单
    pub manifest: TemplateManifest,
    /// 作者 ID
    pub author_id: AuthorId,
    /// 作者名称
    #[serde(default)]
    pub author_name: String,
    /// 定价
    #[serde(default)]
    pub pricing: PricingType,
    /// 价格（分）
    #[serde(default)]
    pub price_cents: u64,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 审核状态
    #[serde(default)]
    pub review_status: ReviewStatus,
    /// 提交时间
    pub submitted_at: DateTime<Utc>,
    /// 自动评估结果
    #[serde(default)]
    pub auto_review: Option<AutoReviewResult>,
    /// 人工审核结果
    #[serde(default)]
    pub manual_review: Option<ManualReviewResult>,
    /// 拒绝原因
    #[serde(default)]
    pub reject_reason: Option<String>,
}

impl TemplateSubmission {
    /// 创建新提交
    pub fn new(manifest: TemplateManifest, author_id: impl Into<String>) -> Self {
        Self {
            id: generate_submission_id(),
            manifest,
            author_id: author_id.into(),
            author_name: String::new(),
            pricing: PricingType::Free,
            price_cents: 0,
            tags: vec![],
            review_status: ReviewStatus::Draft,
            submitted_at: Utc::now(),
            auto_review: None,
            manual_review: None,
            reject_reason: None,
        }
    }

    /// 设置作者名称
    pub fn with_author_name(mut self, name: impl Into<String>) -> Self {
        self.author_name = name.into();
        self
    }

    /// 设置定价
    pub fn with_pricing(mut self, pricing: PricingType, price_cents: u64) -> Self {
        self.pricing = pricing;
        self.price_cents = if pricing == PricingType::Free { 0 } else { price_cents };
        self
    }

    /// 添加标签
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        let t = tag.into();
        if !self.tags.contains(&t) {
            self.tags.push(t);
        }
        self
    }

    /// 提交审核
    pub fn submit(&mut self) -> Result<()> {
        if self.review_status != ReviewStatus::Draft {
            return Err(TeamError::Other("仅草稿状态可提交".into()));
        }
        self.manifest.validate()?;
        self.review_status = ReviewStatus::Submitted;
        self.submitted_at = Utc::now();
        Ok(())
    }

    /// 开始自动评估
    pub fn start_auto_review(&mut self) -> Result<()> {
        if self.review_status != ReviewStatus::Submitted {
            return Err(TeamError::Other("仅已提交状态可开始自动评估".into()));
        }
        self.review_status = ReviewStatus::AutoReviewing;
        Ok(())
    }

    /// 完成自动评估
    pub fn complete_auto_review(&mut self, result: AutoReviewResult) -> Result<()> {
        if self.review_status != ReviewStatus::AutoReviewing {
            return Err(TeamError::Other("仅自动评估中状态可完成评估".into()));
        }
        self.auto_review = Some(result);
        // 自动评估通过则进入人工审核
        if let Some(ref r) = self.auto_review {
            if r.passed {
                self.review_status = ReviewStatus::PendingManual;
            } else {
                self.review_status = ReviewStatus::Rejected;
                self.reject_reason = Some(r.summary.clone());
            }
        }
        Ok(())
    }

    /// 完成人工审核
    pub fn complete_manual_review(&mut self, result: ManualReviewResult) -> Result<()> {
        if self.review_status != ReviewStatus::PendingManual {
            return Err(TeamError::Other("仅待人工审核状态可完成人工审核".into()));
        }
        self.manual_review = Some(result.clone());
        if result.approved {
            self.review_status = ReviewStatus::Approved;
        } else {
            self.review_status = ReviewStatus::Rejected;
            self.reject_reason = Some(result.comment.clone());
        }
        Ok(())
    }

    /// 下架
    pub fn delist(&mut self, reason: impl Into<String>) {
        self.review_status = ReviewStatus::Delisted;
        self.reject_reason = Some(reason.into());
    }

    /// 是否通过审核
    pub fn is_approved(&self) -> bool {
        self.review_status == ReviewStatus::Approved
    }
}

/// 生成提交 ID
fn generate_submission_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let ts = Utc::now().timestamp_millis();
    format!("sub_{ts}_{n:06}")
}

/// 自动评估维度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoReviewDimension {
    /// 维度名称（如 accuracy / safety / usefulness）
    pub name: String,
    /// 评分（0.0 - 10.0）
    pub score: f32,
    /// 说明
    #[serde(default)]
    pub comment: String,
}

/// 自动评估结果（LLM-as-Judge）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoReviewResult {
    /// 评估维度
    pub dimensions: Vec<AutoReviewDimension>,
    /// 总分（0.0 - 10.0）
    pub total_score: f32,
    /// 是否通过（>= 6.0 视为通过）
    pub passed: bool,
    /// 总结
    pub summary: String,
    /// 评估时间
    pub reviewed_at: DateTime<Utc>,
}

impl AutoReviewResult {
    /// 通过阈值
    pub const PASS_THRESHOLD: f32 = 6.0;

    /// 创建评估结果
    pub fn new(dimensions: Vec<AutoReviewDimension>, summary: impl Into<String>) -> Self {
        let total_score = if dimensions.is_empty() {
            0.0
        } else {
            dimensions.iter().map(|d| d.score).sum::<f32>() / dimensions.len() as f32
        };
        let passed = total_score >= Self::PASS_THRESHOLD;
        Self {
            dimensions,
            total_score,
            passed,
            summary: summary.into(),
            reviewed_at: Utc::now(),
        }
    }

    /// 创建模拟通过结果（用于测试 / MVP）
    pub fn mock_passed() -> Self {
        Self::new(
            vec![
                AutoReviewDimension {
                    name: "accuracy".into(),
                    score: 8.5,
                    comment: "员工定义准确，话术示例合理".into(),
                },
                AutoReviewDimension {
                    name: "safety".into(),
                    score: 9.0,
                    comment: "合规规则完整，无禁用词".into(),
                },
                AutoReviewDimension {
                    name: "usefulness".into(),
                    score: 7.5,
                    comment: "评估用例覆盖主要场景".into(),
                },
            ],
            "自动评估通过：模板质量良好",
        )
    }

    /// 创建模拟拒绝结果
    pub fn mock_rejected() -> Self {
        Self::new(
            vec![
                AutoReviewDimension {
                    name: "accuracy".into(),
                    score: 4.0,
                    comment: "员工定义不完整".into(),
                },
                AutoReviewDimension {
                    name: "safety".into(),
                    score: 3.0,
                    comment: "缺少合规规则".into(),
                },
                AutoReviewDimension {
                    name: "usefulness".into(),
                    score: 5.0,
                    comment: "评估用例不足".into(),
                },
            ],
            "自动评估未通过：模板质量不达标",
        )
    }
}

/// 人工审核结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualReviewResult {
    /// 审核员 ID
    pub reviewer_id: String,
    /// 是否通过
    pub approved: bool,
    /// 审核意见
    pub comment: String,
    /// 审核时间
    pub reviewed_at: DateTime<Utc>,
}

impl ManualReviewResult {
    /// 创建通过结果
    pub fn approve(reviewer_id: impl Into<String>, comment: impl Into<String>) -> Self {
        Self {
            reviewer_id: reviewer_id.into(),
            approved: true,
            comment: comment.into(),
            reviewed_at: Utc::now(),
        }
    }

    /// 创建拒绝结果
    pub fn reject(reviewer_id: impl Into<String>, comment: impl Into<String>) -> Self {
        Self {
            reviewer_id: reviewer_id.into(),
            approved: false,
            comment: comment.into(),
            reviewed_at: Utc::now(),
        }
    }
}

/// 模板市场（聚合所有公开模板）
#[derive(Debug, Default)]
pub struct TemplateMarketplace {
    /// 市场条目（按 ID 索引）
    entries: HashMap<MarketId, TemplateMarketEntry>,
    /// 提交记录（按 ID 索引）
    submissions: HashMap<String, TemplateSubmission>,
}

impl TemplateMarketplace {
    /// 创建空市场
    pub fn new() -> Self {
        Self::default()
    }

    /// 发布条目（审核通过后加入市场）
    pub fn publish(&mut self, entry: TemplateMarketEntry) -> Result<MarketId> {
        if !entry.is_approved() {
            return Err(TeamError::Other("仅审核通过的模板可发布".into()));
        }
        let id = entry.id.clone();
        self.entries.insert(id.clone(), entry);
        Ok(id)
    }

    /// 从提交创建并发布条目
    pub fn publish_from_submission(&mut self, submission: &TemplateSubmission) -> Result<MarketId> {
        if !submission.is_approved() {
            return Err(TeamError::Other("仅审核通过的提交可发布".into()));
        }
        let mut entry = TemplateMarketEntry::from_manifest(&submission.manifest, &submission.author_id);
        entry.author_name = submission.author_name.clone();
        entry.pricing = submission.pricing;
        entry.price_cents = submission.price_cents;
        entry.tags = submission.tags.clone();
        entry.review_status = ReviewStatus::Approved;
        self.publish(entry)
    }

    /// 下架条目
    pub fn delist(&mut self, market_id: &str, reason: impl Into<String>) -> Result<()> {
        let reason = reason.into();
        let entry = self
            .entries
            .get_mut(market_id)
            .ok_or_else(|| TeamError::Other(format!("市场条目不存在: {market_id}")))?;
        entry.review_status = ReviewStatus::Delisted;
        // 不直接删除，保留下架记录；下架原因通过 eprintln 输出便于排查
        eprintln!("[marketplace] 下架 {}: {}", market_id, reason);
        Ok(())
    }

    /// 浏览所有已上架条目
    pub fn list_listed(&self) -> Vec<&TemplateMarketEntry> {
        self.entries
            .values()
            .filter(|e| e.is_listed())
            .collect()
    }

    /// 浏览全部条目（含未上架）
    pub fn list_all(&self) -> Vec<&TemplateMarketEntry> {
        self.entries.values().collect()
    }

    /// 按 ID 获取条目
    pub fn get(&self, market_id: &str) -> Option<&TemplateMarketEntry> {
        self.entries.get(market_id)
    }

    /// 按 ID 获取条目（可变）
    pub fn get_mut(&mut self, market_id: &str) -> Option<&mut TemplateMarketEntry> {
        self.entries.get_mut(market_id)
    }

    /// 按行业筛选
    pub fn by_industry(&self, industry: &str) -> Vec<&TemplateMarketEntry> {
        self.entries
            .values()
            .filter(|e| e.is_listed() && e.industry == industry)
            .collect()
    }

    /// 按标签筛选
    pub fn by_tag(&self, tag: &str) -> Vec<&TemplateMarketEntry> {
        self.entries
            .values()
            .filter(|e| e.is_listed() && e.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// 按关键词搜索（名称 + 描述 + 标签）
    pub fn search(&self, query: &str) -> Vec<&TemplateMarketEntry> {
        let q = query.to_lowercase();
        self.entries
            .values()
            .filter(|e| {
                if !e.is_listed() {
                    return false;
                }
                e.name.to_lowercase().contains(&q)
                    || e.description.to_lowercase().contains(&q)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// 按评分排序（降序）
    pub fn top_rated(&self, limit: usize) -> Vec<&TemplateMarketEntry> {
        let mut listed: Vec<&TemplateMarketEntry> = self.list_listed();
        listed.sort_by(|a, b| {
            b.rating.average.partial_cmp(&a.rating.average).unwrap_or(std::cmp::Ordering::Equal)
        });
        listed.into_iter().take(limit).collect()
    }

    /// 按下载量排序（降序）
    pub fn top_downloaded(&self, limit: usize) -> Vec<&TemplateMarketEntry> {
        let mut listed: Vec<&TemplateMarketEntry> = self.list_listed();
        listed.sort_by(|a, b| b.download_count.cmp(&a.download_count));
        listed.into_iter().take(limit).collect()
    }

    /// 记录下载
    pub fn record_download(&mut self, market_id: &str) -> Result<()> {
        let entry = self
            .entries
            .get_mut(market_id)
            .ok_or_else(|| TeamError::Other(format!("市场条目不存在: {market_id}")))?;
        entry.increment_downloads();
        Ok(())
    }

    /// 添加评分
    pub fn add_rating(&mut self, market_id: &str, rating: TemplateRating) -> Result<()> {
        let entry = self
            .entries
            .get_mut(market_id)
            .ok_or_else(|| TeamError::Other(format!("市场条目不存在: {market_id}")))?;
        entry.add_rating(rating);
        Ok(())
    }

    /// 提交模板（创建 submission）
    pub fn submit(&mut self, submission: TemplateSubmission) -> Result<String> {
        let id = submission.id.clone();
        self.submissions.insert(id.clone(), submission);
        Ok(id)
    }

    /// 获取提交
    pub fn get_submission(&self, submission_id: &str) -> Option<&TemplateSubmission> {
        self.submissions.get(submission_id)
    }

    /// 获取提交（可变）
    pub fn get_submission_mut(&mut self, submission_id: &str) -> Option<&mut TemplateSubmission> {
        self.submissions.get_mut(submission_id)
    }

    /// 列出某作者的全部提交
    pub fn submissions_by_author(&self, author_id: &str) -> Vec<&TemplateSubmission> {
        self.submissions
            .values()
            .filter(|s| s.author_id == author_id)
            .collect()
    }

    /// 条目总数
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// 提交总数
    pub fn submission_count(&self) -> usize {
        self.submissions.len()
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> TemplateManifest {
        let yaml = r#"
id: test-template
name: 测试模板
version: "1.0.0"
industry: ecommerce
description: 测试用模板
employees:
  - a
  - b
"#;
        TemplateManifest::from_yaml(yaml).unwrap()
    }

    #[test]
    fn test_pricing_type_display() {
        assert_eq!(PricingType::Free.to_string(), "free");
        assert_eq!(PricingType::Paid.to_string(), "paid");
    }

    #[test]
    fn test_pricing_type_default() {
        assert_eq!(PricingType::default(), PricingType::Free);
    }

    #[test]
    fn test_review_status_display() {
        assert_eq!(ReviewStatus::Draft.to_string(), "draft");
        assert_eq!(ReviewStatus::Approved.to_string(), "approved");
        assert_eq!(ReviewStatus::Rejected.to_string(), "rejected");
    }

    #[test]
    fn test_review_status_default() {
        assert_eq!(ReviewStatus::default(), ReviewStatus::Draft);
    }

    #[test]
    fn test_template_rating_valid() {
        let r = TemplateRating::new("user_1", 5, "很好").unwrap();
        assert_eq!(r.stars, 5);
        assert_eq!(r.comment, "很好");
    }

    #[test]
    fn test_template_rating_invalid_zero() {
        let r = TemplateRating::new("user_1", 0, "");
        assert!(r.is_err());
    }

    #[test]
    fn test_template_rating_invalid_six() {
        let r = TemplateRating::new("user_1", 6, "");
        assert!(r.is_err());
    }

    #[test]
    fn test_rating_summary_empty() {
        let s = RatingSummary::from_ratings(&[]);
        assert_eq!(s.count, 0);
        assert_eq!(s.average, 0.0);
    }

    #[test]
    fn test_rating_summary_calculation() {
        let ratings = vec![
            TemplateRating::new("u1", 5, "").unwrap(),
            TemplateRating::new("u2", 4, "").unwrap(),
            TemplateRating::new("u3", 3, "").unwrap(),
        ];
        let s = RatingSummary::from_ratings(&ratings);
        assert_eq!(s.count, 3);
        assert!((s.average - 4.0).abs() < 0.01);
        assert_eq!(s.stars_count[4], 1); // 5 星
        assert_eq!(s.stars_count[3], 1); // 4 星
        assert_eq!(s.stars_count[2], 1); // 3 星
    }

    #[test]
    fn test_market_entry_creation() {
        let entry = TemplateMarketEntry::new("test-tpl", "测试模板", "author_1");
        assert_eq!(entry.template_id, "test-tpl");
        assert_eq!(entry.name, "测试模板");
        assert_eq!(entry.author_id, "author_1");
        assert_eq!(entry.pricing, PricingType::Free);
        assert_eq!(entry.review_status, ReviewStatus::Draft);
        assert!(entry.id.starts_with("mkt_"));
    }

    #[test]
    fn test_market_entry_from_manifest() {
        let m = sample_manifest();
        let entry = TemplateMarketEntry::from_manifest(&m, "author_1");
        assert_eq!(entry.template_id, "test-template");
        assert_eq!(entry.name, "测试模板");
        assert_eq!(entry.version, "1.0.0");
        assert_eq!(entry.industry, "ecommerce");
        assert_eq!(entry.employee_count, 2);
    }

    #[test]
    fn test_market_entry_with_tag() {
        let entry = TemplateMarketEntry::new("t", "T", "a")
            .with_tag("热门")
            .with_tag("推荐")
            .with_tag("热门"); // 重复
        assert_eq!(entry.tags.len(), 2);
    }

    #[test]
    fn test_market_entry_with_pricing_free() {
        let entry = TemplateMarketEntry::new("t", "T", "a")
            .with_pricing(PricingType::Free, 0);
        assert_eq!(entry.pricing, PricingType::Free);
        assert_eq!(entry.price_cents, 0);
        assert_eq!(entry.price_yuan(), 0.0);
    }

    #[test]
    fn test_market_entry_with_pricing_paid() {
        let entry = TemplateMarketEntry::new("t", "T", "a")
            .with_pricing(PricingType::Paid, 9900); // 99 元
        assert_eq!(entry.pricing, PricingType::Paid);
        assert_eq!(entry.price_cents, 9900);
        assert!((entry.price_yuan() - 99.0).abs() < 0.01);
    }

    #[test]
    fn test_market_entry_add_rating() {
        let mut entry = TemplateMarketEntry::new("t", "T", "a");
        entry.add_rating(TemplateRating::new("u1", 5, "").unwrap());
        entry.add_rating(TemplateRating::new("u2", 4, "").unwrap());
        assert_eq!(entry.rating.count, 2);
        assert!((entry.rating.average - 4.5).abs() < 0.01);
    }

    #[test]
    fn test_market_entry_increment_downloads() {
        let mut entry = TemplateMarketEntry::new("t", "T", "a");
        assert_eq!(entry.download_count, 0);
        entry.increment_downloads();
        entry.increment_downloads();
        assert_eq!(entry.download_count, 2);
    }

    #[test]
    fn test_market_entry_is_approved() {
        let mut entry = TemplateMarketEntry::new("t", "T", "a");
        assert!(!entry.is_approved());
        entry.review_status = ReviewStatus::Approved;
        assert!(entry.is_approved());
        assert!(entry.is_listed());
    }

    #[test]
    fn test_market_entry_is_listed_only_approved() {
        let mut entry = TemplateMarketEntry::new("t", "T", "a");
        entry.review_status = ReviewStatus::PendingManual;
        assert!(!entry.is_listed());
        entry.review_status = ReviewStatus::Approved;
        assert!(entry.is_listed());
        entry.review_status = ReviewStatus::Delisted;
        assert!(!entry.is_listed());
    }

    #[test]
    fn test_submission_creation() {
        let m = sample_manifest();
        let sub = TemplateSubmission::new(m, "author_1");
        assert_eq!(sub.author_id, "author_1");
        assert_eq!(sub.review_status, ReviewStatus::Draft);
        assert!(sub.id.starts_with("sub_"));
    }

    #[test]
    fn test_submission_with_author_name() {
        let m = sample_manifest();
        let sub = TemplateSubmission::new(m, "a").with_author_name("张三");
        assert_eq!(sub.author_name, "张三");
    }

    #[test]
    fn test_submission_with_pricing() {
        let m = sample_manifest();
        let sub = TemplateSubmission::new(m, "a").with_pricing(PricingType::Paid, 5000);
        assert_eq!(sub.pricing, PricingType::Paid);
        assert_eq!(sub.price_cents, 5000);
    }

    #[test]
    fn test_submission_with_tag() {
        let m = sample_manifest();
        let sub = TemplateSubmission::new(m, "a").with_tag("推荐");
        assert_eq!(sub.tags.len(), 1);
    }

    #[test]
    fn test_submission_submit_from_draft() {
        let m = sample_manifest();
        let mut sub = TemplateSubmission::new(m, "a");
        sub.submit().unwrap();
        assert_eq!(sub.review_status, ReviewStatus::Submitted);
    }

    #[test]
    fn test_submission_submit_twice_fails() {
        let m = sample_manifest();
        let mut sub = TemplateSubmission::new(m, "a");
        sub.submit().unwrap();
        assert!(sub.submit().is_err());
    }

    #[test]
    fn test_submission_start_auto_review() {
        let m = sample_manifest();
        let mut sub = TemplateSubmission::new(m, "a");
        sub.submit().unwrap();
        sub.start_auto_review().unwrap();
        assert_eq!(sub.review_status, ReviewStatus::AutoReviewing);
    }

    #[test]
    fn test_submission_start_auto_review_wrong_status() {
        let m = sample_manifest();
        let mut sub = TemplateSubmission::new(m, "a");
        assert!(sub.start_auto_review().is_err());
    }

    #[test]
    fn test_submission_complete_auto_review_passed() {
        let m = sample_manifest();
        let mut sub = TemplateSubmission::new(m, "a");
        sub.submit().unwrap();
        sub.start_auto_review().unwrap();
        sub.complete_auto_review(AutoReviewResult::mock_passed()).unwrap();
        assert_eq!(sub.review_status, ReviewStatus::PendingManual);
    }

    #[test]
    fn test_submission_complete_auto_review_rejected() {
        let m = sample_manifest();
        let mut sub = TemplateSubmission::new(m, "a");
        sub.submit().unwrap();
        sub.start_auto_review().unwrap();
        sub.complete_auto_review(AutoReviewResult::mock_rejected()).unwrap();
        assert_eq!(sub.review_status, ReviewStatus::Rejected);
        assert!(sub.reject_reason.is_some());
    }

    #[test]
    fn test_submission_complete_manual_review_approved() {
        let m = sample_manifest();
        let mut sub = TemplateSubmission::new(m, "a");
        sub.submit().unwrap();
        sub.start_auto_review().unwrap();
        sub.complete_auto_review(AutoReviewResult::mock_passed()).unwrap();
        sub.complete_manual_review(ManualReviewResult::approve("reviewer_1", "通过")).unwrap();
        assert_eq!(sub.review_status, ReviewStatus::Approved);
        assert!(sub.is_approved());
    }

    #[test]
    fn test_submission_complete_manual_review_rejected() {
        let m = sample_manifest();
        let mut sub = TemplateSubmission::new(m, "a");
        sub.submit().unwrap();
        sub.start_auto_review().unwrap();
        sub.complete_auto_review(AutoReviewResult::mock_passed()).unwrap();
        sub.complete_manual_review(ManualReviewResult::reject("reviewer_1", "不通过")).unwrap();
        assert_eq!(sub.review_status, ReviewStatus::Rejected);
    }

    #[test]
    fn test_submission_delist() {
        let m = sample_manifest();
        let mut sub = TemplateSubmission::new(m, "a");
        sub.delist("违规内容");
        assert_eq!(sub.review_status, ReviewStatus::Delisted);
    }

    #[test]
    fn test_auto_review_result_calculation() {
        let result = AutoReviewResult::new(
            vec![
                AutoReviewDimension { name: "a".into(), score: 8.0, comment: "".into() },
                AutoReviewDimension { name: "b".into(), score: 6.0, comment: "".into() },
            ],
            "测试",
        );
        assert!((result.total_score - 7.0).abs() < 0.01);
        assert!(result.passed);
    }

    #[test]
    fn test_auto_review_result_below_threshold() {
        let result = AutoReviewResult::new(
            vec![
                AutoReviewDimension { name: "a".into(), score: 4.0, comment: "".into() },
                AutoReviewDimension { name: "b".into(), score: 5.0, comment: "".into() },
            ],
            "测试",
        );
        assert!((result.total_score - 4.5).abs() < 0.01);
        assert!(!result.passed);
    }

    #[test]
    fn test_auto_review_result_empty() {
        let result = AutoReviewResult::new(vec![], "空");
        assert_eq!(result.total_score, 0.0);
        assert!(!result.passed);
    }

    #[test]
    fn test_auto_review_mock_passed() {
        let r = AutoReviewResult::mock_passed();
        assert!(r.passed);
        assert!(r.total_score >= AutoReviewResult::PASS_THRESHOLD);
        assert_eq!(r.dimensions.len(), 3);
    }

    #[test]
    fn test_auto_review_mock_rejected() {
        let r = AutoReviewResult::mock_rejected();
        assert!(!r.passed);
        assert!(r.total_score < AutoReviewResult::PASS_THRESHOLD);
    }

    #[test]
    fn test_manual_review_approve() {
        let r = ManualReviewResult::approve("rev_1", "通过");
        assert!(r.approved);
        assert_eq!(r.reviewer_id, "rev_1");
    }

    #[test]
    fn test_manual_review_reject() {
        let r = ManualReviewResult::reject("rev_1", "不通过");
        assert!(!r.approved);
    }

    #[test]
    fn test_marketplace_new() {
        let m = TemplateMarketplace::new();
        assert_eq!(m.entry_count(), 0);
        assert_eq!(m.submission_count(), 0);
    }

    #[test]
    fn test_marketplace_publish_approved() {
        let mut m = TemplateMarketplace::new();
        let mut entry = TemplateMarketEntry::new("t", "T", "a");
        entry.review_status = ReviewStatus::Approved;
        let id = m.publish(entry).unwrap();
        assert_eq!(m.entry_count(), 1);
        assert!(m.get(&id).is_some());
    }

    #[test]
    fn test_marketplace_publish_not_approved_fails() {
        let mut m = TemplateMarketplace::new();
        let entry = TemplateMarketEntry::new("t", "T", "a"); // Draft 状态
        assert!(m.publish(entry).is_err());
    }

    #[test]
    fn test_marketplace_publish_from_submission() {
        let mut m = TemplateMarketplace::new();
        let manifest = sample_manifest();
        let mut sub = TemplateSubmission::new(manifest, "a");
        sub.submit().unwrap();
        sub.start_auto_review().unwrap();
        sub.complete_auto_review(AutoReviewResult::mock_passed()).unwrap();
        sub.complete_manual_review(ManualReviewResult::approve("rev", "通过")).unwrap();

        let id = m.publish_from_submission(&sub).unwrap();
        assert_eq!(m.entry_count(), 1);
        let entry = m.get(&id).unwrap();
        assert_eq!(entry.template_id, "test-template");
        assert!(entry.is_listed());
    }

    #[test]
    fn test_marketplace_publish_from_submission_not_approved() {
        let mut m = TemplateMarketplace::new();
        let manifest = sample_manifest();
        let sub = TemplateSubmission::new(manifest, "a"); // Draft
        assert!(m.publish_from_submission(&sub).is_err());
    }

    #[test]
    fn test_marketplace_list_listed() {
        let mut m = TemplateMarketplace::new();
        let mut e1 = TemplateMarketEntry::new("t1", "T1", "a");
        e1.review_status = ReviewStatus::Approved;
        let mut e2 = TemplateMarketEntry::new("t2", "T2", "a");
        e2.review_status = ReviewStatus::Approved;
        // e3 不发布（Draft 状态无法发布）
        m.publish(e1).unwrap();
        m.publish(e2).unwrap();

        let listed = m.list_listed();
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn test_marketplace_by_industry() {
        let mut m = TemplateMarketplace::new();
        let mut e1 = TemplateMarketEntry::new("t1", "T1", "a");
        e1.review_status = ReviewStatus::Approved;
        e1.industry = "ecommerce".into();
        let mut e2 = TemplateMarketEntry::new("t2", "T2", "a");
        e2.review_status = ReviewStatus::Approved;
        e2.industry = "finance".into();
        m.publish(e1).unwrap();
        m.publish(e2).unwrap();

        let ecommerce = m.by_industry("ecommerce");
        assert_eq!(ecommerce.len(), 1);
        let finance = m.by_industry("finance");
        assert_eq!(finance.len(), 1);
        let education = m.by_industry("education");
        assert_eq!(education.len(), 0);
    }

    #[test]
    fn test_marketplace_by_tag() {
        let mut m = TemplateMarketplace::new();
        let mut e1 = TemplateMarketEntry::new("t1", "T1", "a")
            .with_tag("热门");
        e1.review_status = ReviewStatus::Approved;
        let mut e2 = TemplateMarketEntry::new("t2", "T2", "a")
            .with_tag("推荐");
        e2.review_status = ReviewStatus::Approved;
        m.publish(e1).unwrap();
        m.publish(e2).unwrap();

        assert_eq!(m.by_tag("热门").len(), 1);
        assert_eq!(m.by_tag("推荐").len(), 1);
        assert_eq!(m.by_tag("不存在").len(), 0);
    }

    #[test]
    fn test_marketplace_search() {
        let mut m = TemplateMarketplace::new();
        let mut e1 = TemplateMarketEntry::new("t1", "穿搭模板", "a")
            .with_description("电商穿搭行业");
        e1.review_status = ReviewStatus::Approved;
        let mut e2 = TemplateMarketEntry::new("t2", "金融模板", "a")
            .with_description("证券股票行业");
        e2.review_status = ReviewStatus::Approved;
        m.publish(e1).unwrap();
        m.publish(e2).unwrap();

        let results = m.search("穿搭");
        assert_eq!(results.len(), 1);
        let results = m.search("模板");
        assert_eq!(results.len(), 2);
        let results = m.search("不存在");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_marketplace_search_case_insensitive() {
        let mut m = TemplateMarketplace::new();
        let mut e = TemplateMarketEntry::new("t", "Ecommerce Template", "a");
        e.review_status = ReviewStatus::Approved;
        m.publish(e).unwrap();

        let results = m.search("ECOMMERCE");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_marketplace_top_rated() {
        let mut m = TemplateMarketplace::new();
        let mut e1 = TemplateMarketEntry::new("t1", "T1", "a");
        e1.review_status = ReviewStatus::Approved;
        e1.add_rating(TemplateRating::new("u", 5, "").unwrap());
        let mut e2 = TemplateMarketEntry::new("t2", "T2", "a");
        e2.review_status = ReviewStatus::Approved;
        e2.add_rating(TemplateRating::new("u", 3, "").unwrap());
        m.publish(e1).unwrap();
        m.publish(e2).unwrap();

        let top = m.top_rated(10);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].name, "T1"); // 5 星在前
    }

    #[test]
    fn test_marketplace_top_downloaded() {
        let mut m = TemplateMarketplace::new();
        let mut e1 = TemplateMarketEntry::new("t1", "T1", "a");
        e1.review_status = ReviewStatus::Approved;
        e1.download_count = 100;
        let mut e2 = TemplateMarketEntry::new("t2", "T2", "a");
        e2.review_status = ReviewStatus::Approved;
        e2.download_count = 500;
        m.publish(e1).unwrap();
        m.publish(e2).unwrap();

        let top = m.top_downloaded(10);
        assert_eq!(top[0].name, "T2"); // 500 下载在前
    }

    #[test]
    fn test_marketplace_record_download() {
        let mut m = TemplateMarketplace::new();
        let mut e = TemplateMarketEntry::new("t", "T", "a");
        e.review_status = ReviewStatus::Approved;
        let id = m.publish(e).unwrap();

        m.record_download(&id).unwrap();
        m.record_download(&id).unwrap();
        let entry = m.get(&id).unwrap();
        assert_eq!(entry.download_count, 2);
    }

    #[test]
    fn test_marketplace_record_download_nonexistent() {
        let mut m = TemplateMarketplace::new();
        assert!(m.record_download("nonexistent").is_err());
    }

    #[test]
    fn test_marketplace_add_rating() {
        let mut m = TemplateMarketplace::new();
        let mut e = TemplateMarketEntry::new("t", "T", "a");
        e.review_status = ReviewStatus::Approved;
        let id = m.publish(e).unwrap();

        m.add_rating(&id, TemplateRating::new("u1", 5, "").unwrap()).unwrap();
        let entry = m.get(&id).unwrap();
        assert_eq!(entry.rating.count, 1);
    }

    #[test]
    fn test_marketplace_delist() {
        let mut m = TemplateMarketplace::new();
        let mut e = TemplateMarketEntry::new("t", "T", "a");
        e.review_status = ReviewStatus::Approved;
        let id = m.publish(e).unwrap();

        m.delist(&id, "违规").unwrap();
        let entry = m.get(&id).unwrap();
        assert_eq!(entry.review_status, ReviewStatus::Delisted);
        assert!(!entry.is_listed());
    }

    #[test]
    fn test_marketplace_submit() {
        let mut m = TemplateMarketplace::new();
        let manifest = sample_manifest();
        let sub = TemplateSubmission::new(manifest, "a");
        let id = m.submit(sub).unwrap();
        assert_eq!(m.submission_count(), 1);
        assert!(m.get_submission(&id).is_some());
    }

    #[test]
    fn test_marketplace_submissions_by_author() {
        let mut m = TemplateMarketplace::new();
        let m1 = sample_manifest();
        let m2 = TemplateManifest::from_yaml(r#"
id: other
name: 其他
version: "1.0.0"
industry: ecommerce
employees:
  - x
"#).unwrap();
        m.submit(TemplateSubmission::new(m1, "author_1")).unwrap();
        m.submit(TemplateSubmission::new(m2, "author_2")).unwrap();

        let by_a1 = m.submissions_by_author("author_1");
        assert_eq!(by_a1.len(), 1);
    }

    #[test]
    fn test_marketplace_full_workflow() {
        // 完整工作流：提交 → 自动评估 → 人工审核 → 发布 → 下载 → 评分
        let mut m = TemplateMarketplace::new();
        let manifest = sample_manifest();
        let mut sub = TemplateSubmission::new(manifest, "author_1")
            .with_author_name("张三")
            .with_pricing(PricingType::Paid, 9900)
            .with_tag("推荐");
        sub.submit().unwrap();
        sub.start_auto_review().unwrap();
        sub.complete_auto_review(AutoReviewResult::mock_passed()).unwrap();
        sub.complete_manual_review(ManualReviewResult::approve("rev_1", "通过")).unwrap();
        assert!(sub.is_approved());

        let market_id = m.publish_from_submission(&sub).unwrap();
        m.record_download(&market_id).unwrap();
        m.add_rating(&market_id, TemplateRating::new("u1", 5, "很好用").unwrap()).unwrap();

        let entry = m.get(&market_id).unwrap();
        assert_eq!(entry.download_count, 1);
        assert_eq!(entry.rating.count, 1);
        assert_eq!(entry.author_name, "张三");
        assert_eq!(entry.pricing, PricingType::Paid);
    }

    #[test]
    fn test_market_entry_unique_ids() {
        let e1 = TemplateMarketEntry::new("t", "T", "a");
        let e2 = TemplateMarketEntry::new("t", "T", "a");
        assert_ne!(e1.id, e2.id);
    }

    #[test]
    fn test_submission_unique_ids() {
        let m = sample_manifest();
        let s1 = TemplateSubmission::new(m.clone(), "a");
        let s2 = TemplateSubmission::new(m, "a");
        assert_ne!(s1.id, s2.id);
    }

    #[test]
    fn test_auto_review_pass_threshold() {
        assert_eq!(AutoReviewResult::PASS_THRESHOLD, 6.0);
    }

    #[test]
    fn test_market_entry_with_download_url() {
        let e = TemplateMarketEntry::new("t", "T", "a")
            .with_download_url("https://example.com/tpl.zip");
        assert_eq!(e.download_url, Some("https://example.com/tpl.zip".to_string()));
    }

    #[test]
    fn test_marketplace_list_all_includes_unlisted() {
        let mut m = TemplateMarketplace::new();
        let mut e1 = TemplateMarketEntry::new("t1", "T1", "a");
        e1.review_status = ReviewStatus::Approved;
        let mut e2 = TemplateMarketEntry::new("t2", "T2", "a");
        e2.review_status = ReviewStatus::Approved;
        m.publish(e1).unwrap();
        m.publish(e2).unwrap();

        // 直接插入一个 Delisted 条目（模拟已下架）
        let mut e3 = TemplateMarketEntry::new("t3", "T3", "a");
        e3.review_status = ReviewStatus::Delisted;
        m.entries.insert(e3.id.clone(), e3);

        // list_listed 只返回已上架
        assert_eq!(m.list_listed().len(), 2);
        // list_all 返回全部（含下架）
        assert_eq!(m.list_all().len(), 3);
    }

    #[test]
    fn test_marketplace_get_mut() {
        let mut m = TemplateMarketplace::new();
        let mut e = TemplateMarketEntry::new("t", "T", "a");
        e.review_status = ReviewStatus::Approved;
        let id = m.publish(e).unwrap();

        let entry = m.get_mut(&id).unwrap();
        entry.download_count = 100;
        assert_eq!(m.get(&id).unwrap().download_count, 100);
    }

    #[test]
    fn test_marketplace_get_submission_mut() {
        let mut m = TemplateMarketplace::new();
        let manifest = sample_manifest();
        let sub = TemplateSubmission::new(manifest, "a");
        let id = sub.id.clone();
        m.submit(sub).unwrap();

        let s = m.get_submission_mut(&id).unwrap();
        s.review_status = ReviewStatus::Submitted;
        assert_eq!(m.get_submission(&id).unwrap().review_status, ReviewStatus::Submitted);
    }
}
