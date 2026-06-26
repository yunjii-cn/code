# work.yunjii.cn

> 状态：待开发  
> 范围：云集工作台矩阵门户 / 统一入口  
> 最近更新：2026-06-24

`work.yunjii.cn` 是云集产品矩阵门户，不是具体产品的重业务站点。

## 定位

- 展示云集 4 条产品线：云集 PC 版、云集 Web 版、云集团队版、AgentWork。
- 帮用户按 Free / Pro / Business / Enterprise 选择合适产品。
- 承接统一登录、SSO、应用启动器和生态路由。
- 后续接入 UM 统一用户数字身份管理系统。

## 首版页面

1. 首页：产品矩阵和选择向导。
2. 价格页：Free / Pro / Business / Enterprise 对比。
3. 下载与入口页：桌面下载、Web 入口、旗舰申请、AgentWork 试用。
4. 登录入口：先占位，后续接 UM/SSO。

## 开发约束

- 可在当前仓库孵化，但构建、部署和内容目录从第一天保持可拆分。
- 不承载 AgentWork 的产品重叙事，AgentWork 专属转化放到 `aw.yunjii.cn/`。
- 不自建独立用户数据库。
- 内容依据根级 [../doc/产品线矩阵.md](../doc/产品线矩阵.md)、[../doc/品牌与命名规范.md](../doc/品牌与命名规范.md) 和 [../doc/双站点开发计划.md](../doc/双站点开发计划.md)。
