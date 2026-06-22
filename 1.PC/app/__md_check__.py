import sys
sys.path.insert(0, r"e:\软件开发\云集智能编程工作站\1.PC\app")
import main
M = main.MainWindow

test_cases = [
    ("""# 标题
普通段落 **粗体** *斜体* `代码`""",
     ["<h2", "<b", "<i", "<code", "标题", "粗体", "斜体", "代码"]),

    ("""- a
- b
1. x
2. y
- c""",
     ["<ul", "</ul>", "<ol", "</ol>", "<li>a</li>", "<li>b</li>", "<li>x</li>", "<li>c</li>"]),

    ("""```python
def hello():
    print(1)
```
end""",
     ["<pre", "def hello", "end", "</pre>"]),

    ("""[link](https://example.com) end""",
     ["<a href", "link", "end"]),
]

for i, (md, expect) in enumerate(test_cases):
    html = M._markdown_to_html(None, md)
    miss = [w for w in expect if w not in html]
    if miss:
        print(f"  case {i+1} MISS: {miss}")
        print(f"  HTML: {html[:300]}")
    else:
        print(f"  case {i+1} OK")

# 关键：h2 不应被 <p> 包裹
html = M._markdown_to_html(None, "# X")
if "<p ><h2>" in html.replace(" ", "") or "<p><h2" in html:
    print("  h2 BAD: still wrapped in <p>")
else:
    print("  h2 OK: not wrapped in <p>")
print("DONE")
