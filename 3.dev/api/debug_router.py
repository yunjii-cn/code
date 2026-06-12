from platformkit.shared import model_router

code_blocks = "\n\n".join([
    "```python\n" + ("x = 1\n" * 30) + "```"
    for _ in range(3)
])
prompt = (
    "请完整设计一个分布式微服务架构，包含 transformer 模型推理、"
    "kubernetes 部署、加密传输、性能优化算法实现" + code_blocks
)
s = model_router.evaluate_complexity(prompt)
print("total:", s.total, "length:", s.length, "keywords:", s.keywords, "structure:", s.structure, "signals:", s.signals)
d = model_router.decide(prompt)
print("tier:", d.tier, "primary:", d.primary.provider)
