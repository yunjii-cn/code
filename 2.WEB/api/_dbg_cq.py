import sys, os, pathlib, tempfile
os.environ['YUNJI_GLOBAL_ROOT'] = str(pathlib.Path(tempfile.gettempdir()) / 'yj_dbg')
sys.path.insert(0, '.')
from platformkit.shared.responsive_core import CodeQualityPatrol, NotificationType
import re
p = pathlib.Path(tempfile.mkdtemp()) / 't'
p.mkdir()
(p / 'main.py').write_text("def hello():\n    # TODO: fix\n    print('x')\n", encoding='utf-8')
cq = CodeQualityPatrol(p)
print('Path.walk exists:', hasattr(p, 'walk'))
print('Python:', sys.version[:6])
notifs = cq.scan()
print('notifs:', len(notifs))
for n in notifs:
    print(' -', n.title, '|', n.severity.value)
