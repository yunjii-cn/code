# AgentWork 品牌规范

> 状态：现行草案  
> 范围：AgentWork 命名 / AW 简称 / Logo 方向  
> 最近更新：2026-06-24

根级品牌规范见 [../../doc/品牌与命名规范.md](../../doc/品牌与命名规范.md)。本文只记录第 4 代 AgentWork 的产品级品牌约束。

## 1. 命名

| 场景 | 使用 |
|---|---|
| 官网标题、销售材料、合同、商标 | AgentWork |
| 中文解释 | 云集智能体工作台 |
| 图标、CLI、技术短标签 | AW / `aw` |
| GitHub 仓库名 | `agentwork` |
| 目录名 | `4.AgentWork/` |

不要把 AW 当作主要市场名称。AW 是短识别符，AgentWork 才是正式品牌。

## 2. 品牌色

AgentWork / AW 的产品主色为 **`#1E6CF0`**。

云集母品牌继续使用红色体系；AW 不使用红色作为产品主色。AW 与母品牌的关系是“继承结构语言，不继承主色”：继承云集 Logo 的三柱记忆点，但用蓝色建立独立产品识别。

| 角色 | 色值 | 用途 |
|---|---|---|
| AW 主色 | `#1E6CF0` | Logo 底色、CTA、关键高亮、Agent 激活态 |
| AW 深蓝 | `#1553C7` | hover、pressed、深色背景强调边框 |
| AW 浅蓝 | `#5B9BFF` | 微光、focus ring、可视化辅助线 |
| 暗色背景 | `#0B0D12` | 官网和产品暗色基底 |
| 表面深灰 | `#121620` | 卡片、导航、面板 |
| 主文字 | `#F8FAFC` | 标题和主要正文 |
| 次文字 | `#A1A8B3` | 说明和辅助信息 |

## 3. Logo 方向

AgentWork 图标采用云集母品牌体系下的子品牌方案：

- 外形：圆角方形 app icon。
- 底色：AW 蓝 `#1E6CF0`。
- 图形：白色几何 AW monogram。
- 继承点：保留云集主 Logo 的“三柱”记忆点。
- 差异点：减少云朵轮廓，不直接复用母品牌红云图标。

首选方案为 **Agent Grid**：用三根竖柱和斜向连接构成抽象 A/W，表达 Agent、Workflow、协作网络。

## 4. 出图顺序

1. `Agent Grid` 主方向。
2. `AW Fold` 备选方向。
3. `Cloud Core AW` 品牌继承方向。

每个方向需要验证：

- 16px favicon 是否能识别。
- 32px/64px app icon 是否清楚。
- `aw.yunjii.cn` 首屏中是否比普通字母 AW 更有整体性。
- 深色背景、浅色背景和单色印刷是否可用。

## 5. 前端变量

AgentWork 前端、`aw.yunjii.cn` 和 `work.yunjii.cn` 的 AW 相关区域使用同一组变量：

```css
:root {
  --aw-primary: #1e6cf0;
  --aw-primary-hover: #1553c7;
  --aw-primary-soft: #5b9bff;
  --aw-primary-glow: rgba(30, 108, 240, 0.28);
  --aw-bg: #0b0d12;
  --aw-surface: #121620;
  --aw-border: rgba(255, 255, 255, 0.10);
  --aw-text: #f8fafc;
  --aw-text-muted: #a1a8b3;
}
```
