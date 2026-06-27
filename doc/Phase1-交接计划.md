# Phase 1 交接计划

> 日期：2026-06-27  
> 状态：Phase 1 本地视觉原型完成，移交线上部署

## 一、交付物清单

### work.yunjii.cn（云集工作台矩阵门户）
| 组件 | 文件 | 状态 |
|------|------|------|
| 首页框架 | `src/App.tsx` | ✅ |
| 顶栏导航 | `src/components/Navbar.tsx` | ✅ Logo 返回、锚点导航 |
| 首屏 Hero | `src/components/Hero.tsx` | ✅ 粒子动画、计数器 |
| 粒子动效 | `src/components/HeroParticles.tsx` | ✅ Canvas 暖色粒子连线 |
| 产品矩阵 | `src/components/ProductMatrix.tsx` | ✅ 4 代卡片、点击进内页 |
| 产品内页 | `src/components/ProductDetailPage.tsx` | ✅ 全屏覆盖、代际详情 |
| 透明定价 | `src/components/PriceComparison.tsx` | ✅ 0/99/199/399 四档 |
| 统一账号 | `src/components/AccountInfo.tsx` | ✅ |
| 下载入口 | `src/components/DownloadSection.tsx` | ✅ |
| 页脚 | `src/components/Footer.tsx` | ✅ 备案号、官网链接 |
| 双主题 | `src/context/ThemeContext.tsx` | ✅ dark/light/system |
| 中英文 | `src/i18n/` | ✅ zh.json + en.json |
| 内容数据 | `src/content/products.ts` | ✅ 产品 + 定价数据 |
| SEO | `index.html` | ✅ OG/description |
| 图标 | `image/ico.png`, `image/icon.ico` | ✅ |

### aw.yunjii.cn（AgentWork 旗舰产品站）
| 组件 | 文件 | 状态 |
|------|------|------|
| 首页框架 | `src/App.tsx` | ✅ |
| 顶栏导航 | `src/components/Navbar.tsx` | ✅ |
| 首屏 Hero | `src/components/Hero.tsx` | ✅ 分层标题、DAG 代码流 |
| 代码流动效 | `src/components/HeroCodeFlow.tsx` | ✅ Canvas 节点+流光 |
| 产品界面轮播 | `src/components/ProductShowcase.tsx` | ✅ 4 页自动切换 |
| 核心能力 | `src/components/Features.tsx` | ✅ 2×2 卡片 |
| 行业模板 | `src/components/Templates.tsx` | ✅ 4 列渐变图标 |
| 技术架构 | `src/components/Architecture.tsx` | ✅ 脉冲节点时间线 |
| 定价方案 | `src/components/Pricing.tsx` | ✅ 399/699/999/1999 四档 |
| CTA 区块 | `src/components/CtaSection.tsx` | ✅ |
| 页脚 | `src/components/Footer.tsx` | ✅ 备案号、官网链接 |
| 双主题 | `src/context/ThemeContext.tsx` | ✅ |
| 中英文 | `src/i18n/` | ✅ zh.json + en.json |
| SEO | `index.html` | ✅ OG/description |

## 二、技术栈

- Vite 7 + React 19 + TypeScript 5
- Tailwind CSS 4 (@tailwindcss/vite)
- framer-motion（动画）
- react-i18next（国际化）
- 纯静态 SPA，无路由库，无后端

## 三、本地启动

```bash
# work.yunjii.cn（端口 5173）
cd work.yunjii.cn
npx vite

# aw.yunjii.cn（端口 5174）
cd aw.yunjii.cn
npx vite
```

## 四、构建产物

```bash
work.yunjii.cn/dist/  → JS ~437KB (gzip ~139KB), CSS ~37KB
aw.yunjii.cn/dist/    → JS ~436KB (gzip ~135KB), CSS ~60KB
```

## 五、线上部署要点

1. **域名**：work.yunjii.cn / aw.yunjii.cn，DNS 指向部署平台
2. **静态部署**：dist/ 目录直接上传到 CDN / OSS / Vercel / CloudStudio
3. **HTTPS**：必须开启
4. **SPA 路由**：配置 fallback 到 index.html（404 → index.html）
5. **备案号**：鄂ICP备2024085021号-1，链接 https://beian.miit.gov.cn/
6. **统计**：接 Google Analytics 或百度统计

## 六、Phase 2 待办

| 优先级 | 事项 | 预估 |
|--------|------|------|
| P0 | 预约演示表单（接入飞书/企微） | 1d |
| P1 | aw 路线图页面 | 0.5d |
| P1 | work 选择向导模块 | 1d |
| P1 | 移动端全面测试 + 截图 review | 0.5d |
| P2 | FAQ 页面 | 0.5d |
| P2 | 微信公众号/朋友圈分享 OG 图片 | 0.5d |
| P3 | site-shared/ 共享品牌层提取 | 2d |

## 七、Git 信息

- 仓库：github.com/yunjii-cn/code + gitee.com/yunjii/code
- 分支：agentwork
- Phase 1 累计 commits：~25 个
