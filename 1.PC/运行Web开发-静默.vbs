' ── 云集智能编程工作站 - Web开发 (静默启动) ──
' 双击此文件即可静默启动后端+前端，服务就绪后自动打开浏览器

Set ws = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")

' ── 路径 ──
ROOT = fso.GetParentFolderName(WScript.ScriptFullName)
APP_DIR = ROOT & "\app"
WEB_DIR = ROOT & "\web"

' ── 检查 Python ──
On Error Resume Next
Set check = ws.Exec("python --version")
If Err.Number <> 0 Then
    ws.Popup "Python 未安装，请先安装 Python。", 0, "云集智能编程工作站", 48
    WScript.Quit 1
End If
On Error GoTo 0

' ── 检查 FastAPI 依赖 ──
Set check = ws.Exec("python -c ""import fastapi; import uvicorn""")
If check.ExitCode <> 0 Then
    If fso.FileExists(APP_DIR & "\uv\uv.exe") Then
        ws.Run """" & APP_DIR & "\uv\uv.exe"" pip install --system fastapi uvicorn", 0, True
    Else
        ws.Run "pip install fastapi uvicorn --quiet", 0, True
    End If
    Set check2 = ws.Exec("python -c ""import fastapi; import uvicorn""")
    If check2.ExitCode <> 0 Then
        ws.Popup "Python 依赖安装失败。", 0, "云集智能编程工作站", 16
        WScript.Quit 1
    End If
End If

' ── 检查 Node.js ──
On Error Resume Next
Set check = ws.Exec("node --version")
If Err.Number <> 0 Then
    ws.Popup "Node.js 未安装，请先安装 Node.js。", 0, "云集智能编程工作站", 48
    WScript.Quit 1
End If
On Error GoTo 0

' ── 检查前端依赖 ──
If Not fso.FolderExists(WEB_DIR & "\node_modules") Then
    ws.Run "cmd /c ""cd /d """ & WEB_DIR & """ && npm install --silent""", 0, True
End If

' ── 启动后端 (窗口隐藏) ──
ws.CurrentDirectory = APP_DIR
ws.Run "cmd /c ""title YunJi-API && python api_main.py --dev --port 18080""", 0, False

' ── 启动前端 (窗口隐藏) ──
ws.CurrentDirectory = WEB_DIR
ws.Run "cmd /c ""title YunJi-Vite && npm run dev""", 0, False

' ── 等待服务就绪 ──
apiReady = False
viteReady = False
For i = 1 To 30
    WScript.Sleep 1000

    If Not apiReady Then
        On Error Resume Next
        Set http = CreateObject("MSXML2.XMLHTTP")
        http.Open "GET", "http://127.0.0.1:18080/api/health", False
        http.setRequestHeader "Connection", "close"
        http.Send
        If Err.Number = 0 And http.Status = 200 Then
            apiReady = True
        End If
        On Error GoTo 0
    End If

    If Not viteReady Then
        On Error Resume Next
        Set http2 = CreateObject("MSXML2.XMLHTTP")
        http2.Open "GET", "http://localhost:5173", False
        http2.setRequestHeader "Connection", "close"
        http2.Send
        If Err.Number = 0 And http2.Status = 200 Then
            viteReady = True
        End If
        On Error GoTo 0
    End If

    If apiReady And viteReady Then Exit For
Next

' ── 打开浏览器 ──
WScript.Sleep 2000
ws.Run "http://localhost:5173"
