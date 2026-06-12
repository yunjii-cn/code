import os
import json
import re
import shutil
from datetime import datetime


def _uuid() -> str:
    import uuid as _u
    return str(_u.uuid4())


class ProjectManager:
    """项目管理器 - 支持多用户数据隔离架构"""

    DEFAULT_USER_ID = "default"

    def __init__(self, app_dir: str, base_dir: str, data_dir: str = None, user_id: str = None):
        self.app_dir = app_dir
        self.base_dir = base_dir
        self.data_root = data_dir if data_dir else os.path.join(base_dir, "data")
        self.user_id = user_id or self.DEFAULT_USER_ID
        self.public_dir = os.path.join(self.data_root, "public")
        self.data_dir = os.path.join(self.data_root, "users", self.user_id)
        self.projects_dir = os.path.join(self.data_dir, "projects")
        self.sessions_dir = os.path.join(self.data_dir, "sessions")
        self.registry_path = os.path.join(self.projects_dir, "registry.json")
        os.makedirs(self.projects_dir, exist_ok=True)
        os.makedirs(self.sessions_dir, exist_ok=True)
        os.makedirs(self.public_dir, exist_ok=True)
        self._registry = self._load_registry()
        self._migrate_old_projects()

    def _get_data_path(self, project_id: str) -> str:
        dp = os.path.join(self.sessions_dir, project_id)
        os.makedirs(dp, exist_ok=True)
        return dp

    def _migrate_old_projects(self):
        old_yunji_projects = os.path.join(os.path.expanduser("~"), ".yunji", "projects")
        if os.path.exists(old_yunji_projects) and not os.path.exists(self.projects_dir):
            try:
                shutil.copytree(old_yunji_projects, self.projects_dir)
            except Exception:
                pass

        old_flat_projects = os.path.join(self.data_root, "projects")
        if os.path.exists(old_flat_projects) and old_flat_projects != self.projects_dir:
            if not os.path.exists(self.projects_dir):
                try:
                    shutil.copytree(old_flat_projects, self.projects_dir)
                except Exception:
                    pass

        old_flat_sessions = os.path.join(self.data_root, "sessions")
        if os.path.exists(old_flat_sessions) and old_flat_sessions != self.sessions_dir:
            if not os.path.exists(self.sessions_dir):
                try:
                    shutil.copytree(old_flat_sessions, self.sessions_dir)
                except Exception:
                    pass

        for pid, info in self._registry.get("projects", {}).items():
            if "data_path" not in info:
                dp = self._get_data_path(pid)
                old_path = info.get("path", "")
                for sub in ["conversations", "memories"]:
                    old_sub = os.path.join(old_path, sub)
                    new_sub = os.path.join(dp, sub)
                    if os.path.exists(old_sub) and not os.path.exists(new_sub):
                        try:
                            shutil.copytree(old_sub, new_sub)
                        except Exception:
                            pass
                old_claude = os.path.join(old_path, "CLAUDE.md")
                new_claude = os.path.join(dp, "CLAUDE.md")
                if os.path.exists(old_claude) and not os.path.exists(new_claude):
                    try:
                        shutil.copy2(old_claude, new_claude)
                    except Exception:
                        pass
                old_mem_dir = os.path.join(old_path, ".claude", "memories")
                new_mem_dir = os.path.join(dp, "memories")
                if os.path.exists(old_mem_dir) and not os.path.exists(new_mem_dir):
                    try:
                        shutil.copytree(old_mem_dir, new_mem_dir)
                    except Exception:
                        pass
                old_pj = os.path.join(old_path, "project.json")
                new_pj = os.path.join(dp, "project.json")
                if os.path.exists(old_pj) and not os.path.exists(new_pj):
                    try:
                        shutil.copy2(old_pj, new_pj)
                    except Exception:
                        pass
                info["data_path"] = dp
                if not info.get("workspace_path"):
                    info["workspace_path"] = old_path
        self._save_registry()

    def get_default_path(self, name: str) -> str:
        safe_name = "".join(c for c in name if c not in r'\/:*?"<>|').strip()
        if not safe_name:
            safe_name = "project"
        candidate = os.path.join(self.projects_dir, safe_name)
        suffix = 1
        while os.path.exists(candidate):
            candidate = os.path.join(self.projects_dir, f"{safe_name}_{suffix}")
            suffix += 1
        return candidate

    def _load_registry(self) -> dict:
        if os.path.exists(self.registry_path):
            try:
                with open(self.registry_path, "r", encoding="utf-8") as f:
                    return json.load(f)
            except Exception:
                pass
        return {"projects": {}, "active_project": None}

    def _save_registry(self):
        with open(self.registry_path, "w", encoding="utf-8") as f:
            json.dump(self._registry, f, ensure_ascii=False, indent=2)

    def list_projects(self) -> list:
        result = []
        for pid, info in self._registry.get("projects", {}).items():
            result.append({"id": pid, **info})
        return result

    def get_active_project(self) -> dict:
        pid = self._registry.get("active_project")
        if pid and pid in self._registry.get("projects", {}):
            return {"id": pid, **self._registry["projects"][pid]}
        return None

    def create_project(self, name: str, workspace_path: str = "") -> dict:
        pid = _uuid()
        data_path = self._get_data_path(pid)
        conv_dir = os.path.join(data_path, "conversations")
        os.makedirs(conv_dir, exist_ok=True)
        mem_dir = os.path.join(data_path, "memories")
        os.makedirs(mem_dir, exist_ok=True)
        ws = workspace_path.strip() if workspace_path else ""
        if ws:
            os.makedirs(ws, exist_ok=True)
        info = {
            "name": name,
            "data_path": data_path,
            "workspace_path": ws,
            "created_at": datetime.now().isoformat(),
            "updated_at": datetime.now().isoformat(),
        }
        self._registry["projects"][pid] = info
        self._registry["active_project"] = pid
        self._save_registry()
        with open(os.path.join(data_path, "project.json"), "w", encoding="utf-8") as f:
            json.dump({"id": pid, **info}, f, ensure_ascii=False, indent=2)
        return {"id": pid, **info}

    def switch_project(self, project_id: str) -> dict:
        if project_id not in self._registry.get("projects", {}):
            return None
        self._registry["active_project"] = project_id
        self._registry["projects"][project_id]["updated_at"] = datetime.now().isoformat()
        self._save_registry()
        return {"id": project_id, **self._registry["projects"][project_id]}

    def rename_project(self, project_id: str, new_name: str) -> bool:
        if project_id not in self._registry.get("projects", {}):
            return False
        self._registry["projects"][project_id]["name"] = new_name
        self._registry["projects"][project_id]["updated_at"] = datetime.now().isoformat()
        self._save_registry()
        dp = self._get_data_path(project_id)
        meta_path = os.path.join(dp, "project.json")
        with open(meta_path, "w", encoding="utf-8") as f:
            json.dump({"id": project_id, **self._registry["projects"][project_id]}, f, ensure_ascii=False, indent=2)
        return True

    def update_project(self, project_id: str, new_name: str = None, new_workspace_path: str = None) -> dict:
        if project_id not in self._registry.get("projects", {}):
            return None
        info = self._registry["projects"][project_id]
        if new_name is not None and new_name.strip():
            info["name"] = new_name.strip()
        if new_workspace_path is not None:
            ws = new_workspace_path.strip()
            if ws:
                os.makedirs(ws, exist_ok=True)
            info["workspace_path"] = ws
        info["updated_at"] = datetime.now().isoformat()
        self._save_registry()
        dp = self._get_data_path(project_id)
        meta_path = os.path.join(dp, "project.json")
        with open(meta_path, "w", encoding="utf-8") as f:
            json.dump({"id": project_id, **info}, f, ensure_ascii=False, indent=2)
        return {"id": project_id, **info}

    def delete_project(self, project_id: str) -> bool:
        if project_id not in self._registry.get("projects", {}):
            return False
        info = self._registry["projects"].pop(project_id)
        if self._registry.get("active_project") == project_id:
            self._registry["active_project"] = None
        self._save_registry()
        return True

    def save_conversation(self, project_id: str, session_id: str, messages: list, title: str = ""):
        if project_id not in self._registry.get("projects", {}):
            return False
        dp = self._get_data_path(project_id)
        conv_dir = os.path.join(dp, "conversations")
        os.makedirs(conv_dir, exist_ok=True)
        conv_path = os.path.join(conv_dir, f"{session_id}.json")
        existing_data = {}
        if os.path.exists(conv_path):
            try:
                with open(conv_path, "r", encoding="utf-8") as f:
                    existing_data = json.load(f)
            except Exception:
                pass
        if not title:
            for m in messages:
                if m.get("role") == "user" and m.get("text", "").strip():
                    title = m["text"].strip()[:50]
                    break
        if not title:
            title = existing_data.get("title", "")
        data = {
            "session_id": session_id,
            "project_id": project_id,
            "title": title,
            "messages": messages,
            "created_at": existing_data.get("created_at", datetime.now().isoformat()),
            "updated_at": datetime.now().isoformat(),
        }
        with open(conv_path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        return True

    def load_conversation(self, project_id: str, session_id: str) -> list:
        if project_id not in self._registry.get("projects", {}):
            return []
        dp = self._get_data_path(project_id)
        conv_path = os.path.join(dp, "conversations", f"{session_id}.json")
        if not os.path.exists(conv_path):
            return []
        try:
            with open(conv_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            return data.get("messages", [])
        except Exception:
            return []

    def list_conversations(self, project_id: str) -> list:
        if project_id not in self._registry.get("projects", {}):
            return []
        dp = self._get_data_path(project_id)
        conv_dir = os.path.join(dp, "conversations")
        if not os.path.exists(conv_dir):
            return []
        result = []
        for fname in os.listdir(conv_dir):
            if fname.endswith(".json"):
                try:
                    with open(os.path.join(conv_dir, fname), "r", encoding="utf-8") as f:
                        data = json.load(f)
                    msgs = data.get("messages", [])
                    title = data.get("title", "")
                    if not title:
                        for m in msgs:
                            if m.get("role") == "user" and m.get("text", "").strip():
                                title = m["text"].strip()[:50]
                                break
                    result.append({
                        "session_id": data.get("session_id", fname[:-5]),
                        "title": title,
                        "updated_at": data.get("updated_at", ""),
                        "created_at": data.get("created_at", ""),
                        "message_count": len(msgs),
                    })
                except Exception:
                    pass
        result.sort(key=lambda c: c.get("updated_at", ""), reverse=True)
        return result

    def copy_conversation(self, source_project_id: str, session_id: str, target_project_id: str) -> bool:
        if source_project_id not in self._registry.get("projects", {}):
            return False
        if target_project_id not in self._registry.get("projects", {}):
            return False
        src_dp = self._get_data_path(source_project_id)
        dst_dp = self._get_data_path(target_project_id)
        src_file = os.path.join(src_dp, "conversations", f"{session_id}.json")
        if not os.path.exists(src_file):
            return False
        dst_dir = os.path.join(dst_dp, "conversations")
        os.makedirs(dst_dir, exist_ok=True)
        try:
            with open(src_file, "r", encoding="utf-8") as f:
                data = json.load(f)
            data["project_id"] = target_project_id
            data["updated_at"] = datetime.now().isoformat()
            with open(os.path.join(dst_dir, f"{session_id}.json"), "w", encoding="utf-8") as f:
                json.dump(data, f, ensure_ascii=False, indent=2)
            return True
        except Exception:
            return False

    def get_project_context(self, project_id: str, max_conversations: int = 3) -> list:
        if project_id not in self._registry.get("projects", {}):
            return []
        convs = self.list_conversations(project_id)
        context = []
        for c in convs[:max_conversations]:
            msgs = self.load_conversation(project_id, c["session_id"])
            if msgs:
                context.extend(msgs)
        return context

    def search_conversations(self, project_id: str, keyword: str) -> list:
        if project_id not in self._registry.get("projects", {}):
            return []
        if not keyword.strip():
            return self.list_conversations(project_id)
        dp = self._get_data_path(project_id)
        conv_dir = os.path.join(dp, "conversations")
        if not os.path.exists(conv_dir):
            return []
        kw = keyword.strip().lower()
        result = []
        for fname in os.listdir(conv_dir):
            if not fname.endswith(".json"):
                continue
            try:
                with open(os.path.join(conv_dir, fname), "r", encoding="utf-8") as f:
                    data = json.load(f)
                msgs = data.get("messages", [])
                title = data.get("title", "")
                matched = False
                snippet = ""
                if kw in title.lower():
                    matched = True
                for m in msgs:
                    text = m.get("text", "").lower()
                    if kw in text:
                        matched = True
                        if not snippet:
                            idx = text.find(kw)
                            start = max(0, idx - 20)
                            end = min(len(m.get("text", "")), idx + len(kw) + 20)
                            snippet = "..." + m.get("text", "")[start:end] + "..."
                if matched:
                    result.append({
                        "session_id": data.get("session_id", fname[:-5]),
                        "title": title,
                        "updated_at": data.get("updated_at", ""),
                        "created_at": data.get("created_at", ""),
                        "message_count": len(msgs),
                        "snippet": snippet,
                    })
            except Exception:
                pass
        result.sort(key=lambda c: c.get("updated_at", ""), reverse=True)
        return result

    def delete_conversation(self, project_id: str, session_id: str) -> bool:
        if project_id not in self._registry.get("projects", {}):
            return False
        dp = self._get_data_path(project_id)
        conv_path = os.path.join(dp, "conversations", f"{session_id}.json")
        if os.path.exists(conv_path):
            try:
                os.remove(conv_path)
                return True
            except Exception:
                return False
        return False

    def rename_conversation(self, project_id: str, session_id: str, new_title: str) -> bool:
        if project_id not in self._registry.get("projects", {}):
            return False
        dp = self._get_data_path(project_id)
        conv_path = os.path.join(dp, "conversations", f"{session_id}.json")
        if not os.path.exists(conv_path):
            return False
        try:
            with open(conv_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            data["title"] = new_title.strip()
            data["updated_at"] = datetime.now().isoformat()
            with open(conv_path, "w", encoding="utf-8") as f:
                json.dump(data, f, ensure_ascii=False, indent=2)
            return True
        except Exception:
            return False

    def get_claude_md(self, project_id: str) -> str:
        parts = []
        dp = self._get_data_path(project_id)
        data_claude = os.path.join(dp, "CLAUDE.md")
        if os.path.exists(data_claude):
            try:
                with open(data_claude, "r", encoding="utf-8") as f:
                    parts.append(f.read())
            except Exception:
                pass
        info = self._registry.get("projects", {}).get(project_id, {})
        ws = info.get("workspace_path", "")
        if ws:
            for candidate in [os.path.join(ws, "CLAUDE.md"), os.path.join(ws, ".claude", "CLAUDE.md")]:
                if os.path.exists(candidate):
                    try:
                        with open(candidate, "r", encoding="utf-8") as f:
                            content = f.read()
                        if content not in parts:
                            parts.append(content)
                    except Exception:
                        pass
                    break
        return "\n\n".join(parts) if parts else ""

    def save_claude_md(self, project_id: str, content: str) -> bool:
        dp = self._get_data_path(project_id)
        target = os.path.join(dp, "CLAUDE.md")
        try:
            with open(target, "w", encoding="utf-8") as f:
                f.write(content)
            return True
        except Exception:
            return False

    def get_global_claude_md(self) -> str:
        home = os.path.expanduser("~")
        path = os.path.join(home, ".claude", "CLAUDE.md")
        if os.path.exists(path):
            try:
                with open(path, "r", encoding="utf-8") as f:
                    return f.read()
            except Exception:
                pass
        return ""

    def save_global_claude_md(self, content: str) -> bool:
        home = os.path.expanduser("~")
        claude_dir = os.path.join(home, ".claude")
        os.makedirs(claude_dir, exist_ok=True)
        path = os.path.join(claude_dir, "CLAUDE.md")
        try:
            with open(path, "w", encoding="utf-8") as f:
                f.write(content)
            return True
        except Exception:
            return False

    def list_memories(self, project_id: str) -> list:
        dp = self._get_data_path(project_id)
        mem_dir = os.path.join(dp, "memories")
        if not os.path.exists(mem_dir):
            return []
        result = []
        for fname in os.listdir(mem_dir):
            if not fname.endswith(".md"):
                continue
            fpath = os.path.join(mem_dir, fname)
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    content = f.read()
                mem_type = "project"
                title = fname[:-3]
                if content.startswith("---"):
                    end = content.find("---", 3)
                    if end > 0:
                        header = content[3:end].strip()
                        for line in header.split("\n"):
                            if line.startswith("type:"):
                                mem_type = line.split(":", 1)[1].strip().strip('"').strip("'")
                            elif line.startswith("title:"):
                                title = line.split(":", 1)[1].strip().strip('"').strip("'")
                result.append({
                    "filename": fname,
                    "title": title,
                    "type": mem_type,
                    "content": content,
                    "updated_at": os.path.getmtime(fpath),
                })
            except Exception:
                pass
        result.sort(key=lambda m: m.get("updated_at", 0), reverse=True)
        return result

    def save_memory(self, project_id: str, filename: str, content: str, mem_type: str = "project") -> bool:
        dp = self._get_data_path(project_id)
        mem_dir = os.path.join(dp, "memories")
        os.makedirs(mem_dir, exist_ok=True)
        if not filename.endswith(".md"):
            filename += ".md"
        fpath = os.path.join(mem_dir, filename)
        try:
            header = f"---\ntype: {mem_type}\ntitle: {filename[:-3]}\nupdated: {datetime.now().isoformat()}\n---\n\n"
            if content.startswith("---"):
                end = content.find("---", 3)
                if end > 0:
                    header = ""
            with open(fpath, "w", encoding="utf-8") as f:
                f.write(header + content if header else content)
            return True
        except Exception:
            return False

    def delete_memory(self, project_id: str, filename: str) -> bool:
        dp = self._get_data_path(project_id)
        fpath = os.path.join(dp, "memories", filename)
        if os.path.exists(fpath):
            try:
                os.remove(fpath)
                return True
            except Exception:
                return False
        return False

    def search_memories(self, project_id: str, keyword: str) -> list:
        all_mems = self.list_memories(project_id)
        if not keyword.strip():
            return all_mems
        kw = keyword.strip().lower()
        scored = []
        for m in all_mems:
            score = 0
            title = m.get("title", "").lower()
            content = m.get("content", "").lower()
            if kw in title:
                score += 10
            if kw in content:
                score += 5
                idx = content.find(kw)
                m["match_snippet"] = m.get("content", "")[max(0, idx - 30):idx + len(kw) + 30]
            if score > 0:
                m["relevance"] = score
                scored.append(m)
        scored.sort(key=lambda x: x.get("relevance", 0), reverse=True)
        return scored

    def auto_extract_memories(self, project_id: str, messages: list) -> list:
        extracted = []
        user_prefs = []
        feedback_items = []
        project_facts = []
        reference_items = []
        pref_patterns = [
            (r"我喜欢", "用户偏好"), (r"我偏好", "用户偏好"), (r"我习惯", "用户偏好"),
            (r"请用", "用户偏好"), (r"我喜欢用", "用户偏好"), (r"我更喜欢", "用户偏好"),
            (r"我通常", "用户偏好"), (r"我的风格", "用户偏好"), (r"我倾向于", "用户偏好"),
            (r"我一般", "用户偏好"), (r"我常用", "用户偏好"), (r"我习惯用", "用户偏好"),
            (r"我比较喜欢", "用户偏好"), (r"我偏好使用", "用户偏好"),
        ]
        feedback_patterns = [
            (r"不要", "反馈纠正"), (r"别这样", "反馈纠正"), (r"不对", "反馈纠正"),
            (r"错了", "反馈纠正"), (r"不是这样的", "反馈纠正"), (r"应该", "反馈纠正"),
            (r"改一下", "反馈纠正"), (r"换个方式", "反馈纠正"), (r"不行", "反馈纠正"),
            (r"这不对", "反馈纠正"), (r"重新来", "反馈纠正"), (r"不对劲", "反馈纠正"),
            (r"有问题", "反馈纠正"), (r"需要修改", "反馈纠正"), (r"请修正", "反馈纠正"),
        ]
        fact_patterns = [
            (r"项目是", "项目上下文"), (r"用的是", "项目上下文"), (r"技术栈是", "项目上下文"),
            (r"框架是", "项目上下文"), (r"基于", "项目上下文"), (r"运行在", "项目上下文"),
            (r"版本是", "项目上下文"), (r"数据库是", "项目上下文"), (r"语言是", "项目上下文"),
            (r"使用的是", "项目上下文"), (r"开发环境", "项目上下文"), (r"部署在", "项目上下文"),
        ]
        ref_patterns = [
            (r"参考", "外部引用"), (r"文档在", "外部引用"), (r"链接是", "外部引用"),
            (r"官方文档", "外部引用"), (r"API文档", "外部引用"), (r"教程在", "外部引用"),
            (r"仓库是", "外部引用"), (r"源码在", "外部引用"), (r"https?://", "外部引用"),
        ]
        for m in messages:
            text = m.get("text", "")
            if not text.strip():
                continue
            role = m.get("role", "")
            if role != "user":
                continue
            for pattern, _ in pref_patterns:
                match = re.search(pattern, text)
                if match:
                    start = max(0, match.start() - 15)
                    end = min(len(text), match.end() + 60)
                    snippet = text[start:end].strip()
                    if snippet not in user_prefs:
                        user_prefs.append(snippet)
                    break
            for pattern, _ in feedback_patterns:
                match = re.search(pattern, text)
                if match:
                    start = max(0, match.start() - 15)
                    end = min(len(text), match.end() + 60)
                    snippet = text[start:end].strip()
                    if snippet not in feedback_items:
                        feedback_items.append(snippet)
                    break
            for pattern, _ in fact_patterns:
                match = re.search(pattern, text)
                if match:
                    start = max(0, match.start() - 15)
                    end = min(len(text), match.end() + 60)
                    snippet = text[start:end].strip()
                    if snippet not in project_facts:
                        project_facts.append(snippet)
                    break
            for pattern, _ in ref_patterns:
                match = re.search(pattern, text, re.IGNORECASE)
                if match:
                    start = max(0, match.start() - 15)
                    end = min(len(text), match.end() + 80)
                    snippet = text[start:end].strip()
                    if snippet not in reference_items:
                        reference_items.append(snippet)
                    break
        if user_prefs:
            content = self._merge_memory_content(project_id, "user-preferences.md", user_prefs)
            self.save_memory(project_id, "user-preferences.md", content, "user")
            extracted.append({"type": "user", "count": len(user_prefs)})
        if feedback_items:
            content = self._merge_memory_content(project_id, "user-feedback.md", feedback_items)
            self.save_memory(project_id, "user-feedback.md", content, "feedback")
            extracted.append({"type": "feedback", "count": len(feedback_items)})
        if project_facts:
            content = self._merge_memory_content(project_id, "project-context.md", project_facts)
            self.save_memory(project_id, "project-context.md", content, "project")
            extracted.append({"type": "project", "count": len(project_facts)})
        if reference_items:
            content = self._merge_memory_content(project_id, "external-references.md", reference_items)
            self.save_memory(project_id, "external-references.md", content, "reference")
            extracted.append({"type": "reference", "count": len(reference_items)})
        return extracted

    def _merge_memory_content(self, project_id: str, filename: str, new_items: list) -> str:
        dp = self._get_data_path(project_id)
        mem_dir = os.path.join(dp, "memories")
        fpath = os.path.join(mem_dir, filename)
        existing_lines = set()
        if os.path.exists(fpath):
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    old_content = f.read()
                body = old_content
                if body.startswith("---"):
                    end = body.find("---", 3)
                    if end > 0:
                        body = body[end + 3:].strip()
                for line in body.split("\n"):
                    stripped = line.strip()
                    if stripped.startswith("- "):
                        existing_lines.add(stripped[2:].strip().lower())
            except Exception:
                pass
        merged = []
        for item in new_items:
            if item.lower() not in existing_lines:
                merged.append(item)
        if not merged:
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    return f.read()
            except Exception:
                pass
        content_lines = []
        for item in new_items:
            content_lines.append(f"- {item}")
        if os.path.exists(fpath):
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    old_content = f.read()
                body = old_content
                if body.startswith("---"):
                    end = body.find("---", 3)
                    if end > 0:
                        body = body[end + 3:].strip()
                for line in body.split("\n"):
                    stripped = line.strip()
                    if stripped.startswith("- "):
                        item_text = stripped[2:].strip()
                        if item_text.lower() not in [m.lower() for m in merged]:
                            content_lines.append(stripped)
            except Exception:
                pass
        return "\n".join(content_lines)

    def get_memory_stats(self, project_id: str) -> dict:
        mems = self.list_memories(project_id)
        stats = {"total": len(mems), "by_type": {}, "total_size": 0}
        for m in mems:
            t = m.get("type", "project")
            stats["by_type"][t] = stats["by_type"].get(t, 0) + 1
            stats["total_size"] += len(m.get("content", ""))
        return stats

    def get_relevant_memories(self, project_id: str, query: str, limit: int = 5) -> list:
        results = self.search_memories(project_id, query)
        return results[:limit]

    def _get_templates_dir(self) -> str:
        return os.path.join(self.public_dir, "templates")

    def save_custom_template(self, name: str, category: str, desc: str, prompt: str, files: str = "") -> bool:
        tpl_dir = self._get_templates_dir()
        os.makedirs(tpl_dir, exist_ok=True)
        safe_name = name.replace(" ", "-").lower()
        fpath = os.path.join(tpl_dir, f"{safe_name}.json")
        tpl = {
            "id": safe_name,
            "name": name,
            "desc": desc,
            "category": category,
            "prompt": prompt,
            "files": files,
            "custom": True,
            "created_at": datetime.now().isoformat(),
        }
        try:
            with open(fpath, "w", encoding="utf-8") as f:
                json.dump(tpl, f, ensure_ascii=False, indent=2)
            return True
        except Exception:
            return False

    def list_custom_templates(self) -> list:
        tpl_dir = self._get_templates_dir()
        if not os.path.exists(tpl_dir):
            return []
        result = []
        for fname in os.listdir(tpl_dir):
            if not fname.endswith(".json"):
                continue
            fpath = os.path.join(tpl_dir, fname)
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    tpl = json.load(f)
                tpl["custom"] = True
                result.append(tpl)
            except Exception:
                pass
        return result

    def delete_custom_template(self, template_id: str) -> bool:
        tpl_dir = self._get_templates_dir()
        fpath = os.path.join(tpl_dir, f"{template_id}.json")
        if os.path.exists(fpath):
            try:
                os.remove(fpath)
                return True
            except Exception:
                return False
        return False

    def create_project_from_template(self, project_id: str, template_id: str) -> dict:
        builtin = {
            "react-app": {"files": {"package.json": '{"name": "react-app", "version": "0.1.0", "scripts": {"dev": "vite", "build": "vite build"}}', "src/App.tsx": 'export default function App() { return <div>Hello React</div>; }', "vite.config.ts": 'import { defineConfig } from "vite";\nimport react from "@vitejs/plugin-react";\nexport default defineConfig({ plugins: [react()] });'}, "init_cmd": "npm install"},
            "vue-app": {"files": {"package.json": '{"name": "vue-app", "version": "0.1.0", "scripts": {"dev": "vite", "build": "vite build"}}', "src/App.vue": '<template><div>Hello Vue</div></template>', "vite.config.ts": 'import { defineConfig } from "vite";\nimport vue from "@vitejs/plugin-vue";\nexport default defineConfig({ plugins: [vue()] });'}, "init_cmd": "npm install"},
            "flask-api": {"files": {"requirements.txt": "flask\nflask-sqlalchemy", "app.py": 'from flask import Flask\napp = Flask(__name__)\n\n@app.route("/")\ndef hello():\n    return "Hello Flask"\n\nif __name__ == "__main__":\n    app.run(debug=True)'}, "init_cmd": "pip install -r requirements.txt"},
            "fastapi-app": {"files": {"requirements.txt": "fastapi\nuvicorn", "main.py": 'from fastapi import FastAPI\napp = FastAPI()\n\n@app.get("/")\ndef hello():\n    return {"msg": "Hello FastAPI"}'}, "init_cmd": "pip install -r requirements.txt"},
            "static-site": {"files": {"index.html": '<!DOCTYPE html>\n<html><head><title>My Site</title></head><body><h1>Hello World</h1></body></html>', "style.css": "body { font-family: sans-serif; }", "script.js": 'console.log("Hello");'}},
            "landing-page": {"files": {"index.html": '<!DOCTYPE html>\n<html><head><title>Landing Page</title><link rel="stylesheet" href="style.css"></head><body><header><h1>Welcome</h1></header><main><section class="hero"><h2>Our Product</h2><p>Description here</p></section></main></body></html>', "style.css": "* { margin: 0; padding: 0; box-sizing: border-box; }\nbody { font-family: sans-serif; }\n.hero { padding: 80px 20px; text-align: center; }"}},
        }
        tpl_dir = self._get_templates_dir()
        custom_fpath = os.path.join(tpl_dir, f"{template_id}.json")
        tpl_data = None
        if os.path.exists(custom_fpath):
            try:
                with open(custom_fpath, "r", encoding="utf-8") as f:
                    tpl_data = json.load(f)
            except Exception:
                pass
        if not tpl_data and template_id in builtin:
            tpl_data = builtin[template_id]
        if not tpl_data:
            return {"success": False, "error": f"模板 {template_id} 不存在"}
        info = self._registry.get("projects", {}).get(project_id, {})
        ws = info.get("workspace_path", "")
        if not ws:
            dp = self._get_data_path(project_id)
            ws = dp
        os.makedirs(ws, exist_ok=True)
        files_created = []
        tpl_files = tpl_data.get("files", {})
        if isinstance(tpl_files, str):
            try:
                tpl_files = json.loads(tpl_files)
            except Exception:
                tpl_files = {}
        for rel_path, content in tpl_files.items():
            fpath = os.path.join(ws, rel_path)
            os.makedirs(os.path.dirname(fpath), exist_ok=True)
            try:
                with open(fpath, "w", encoding="utf-8") as f:
                    f.write(content)
                files_created.append(rel_path)
            except Exception:
                pass
        init_cmd = tpl_data.get("init_cmd", "")
        return {"success": True, "files": files_created, "init_cmd": init_cmd}


def _get_base_dir() -> str:
    import sys
    if getattr(sys, 'frozen', False):
        dev_dir = os.environ.get("YUNJI_DEV_DIR", "")
        if dev_dir:
            return os.path.abspath(dev_dir)
        return os.path.abspath(os.path.dirname(sys.executable))
    return os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))


class ProjectService:
    def __init__(self, app_dir=None, base_dir=None, data_dir=None, user_id=None):
        base = base_dir or _get_base_dir()
        self._manager = ProjectManager(
            app_dir=app_dir or os.path.join(base, "app"),
            base_dir=base,
            data_dir=data_dir or os.path.join(base, "data"),
            user_id=user_id,
        )

    async def list_projects(self):
        return self._manager.list_projects()

    async def get_active_project(self):
        return self._manager.get_active_project()

    async def create_project(self, name: str, workspace_path: str = None):
        return self._manager.create_project(name, workspace_path or "")

    async def switch_project(self, project_id: str):
        return self._manager.switch_project(project_id)

    async def rename_project(self, project_id: str, new_name: str):
        ok = self._manager.rename_project(project_id, new_name)
        return {"ok": ok}

    async def update_project(self, project_id: str, name: str = None, workspace_path: str = None):
        return self._manager.update_project(project_id, name, workspace_path)

    async def delete_project(self, project_id: str):
        ok = self._manager.delete_project(project_id)
        return {"ok": ok}

    async def get_default_path(self, name: str):
        return {"path": self._manager.get_default_path(name)}

    async def list_conversations(self, project_id: str):
        return self._manager.list_conversations(project_id)

    async def load_conversation(self, project_id: str, session_id: str):
        return self._manager.load_conversation(project_id, session_id)

    async def save_conversation(self, project_id: str, session_id: str, messages: list, title: str = None):
        ok = self._manager.save_conversation(project_id, session_id, messages, title or "")
        return {"ok": ok}

    async def copy_conversation(self, source_project_id: str, session_id: str, target_project_id: str):
        ok = self._manager.copy_conversation(source_project_id, session_id, target_project_id)
        return {"ok": ok}

    async def rename_conversation(self, project_id: str, session_id: str, new_title: str):
        ok = self._manager.rename_conversation(project_id, session_id, new_title)
        return {"ok": ok}

    async def delete_conversation(self, project_id: str, session_id: str):
        ok = self._manager.delete_conversation(project_id, session_id)
        return {"ok": ok}

    async def search_conversations(self, project_id: str, keyword: str):
        return self._manager.search_conversations(project_id, keyword)

    async def get_project_context(self, project_id: str):
        return self._manager.get_project_context(project_id)

    async def get_claude_md(self, project_id: str):
        content = self._manager.get_claude_md(project_id)
        return {"content": content}

    async def save_claude_md(self, project_id: str, content: str):
        ok = self._manager.save_claude_md(project_id, content)
        return {"ok": ok}

    async def get_global_claude_md(self):
        content = self._manager.get_global_claude_md()
        return {"content": content}

    async def save_global_claude_md(self, content: str):
        ok = self._manager.save_global_claude_md(content)
        return {"ok": ok}

    async def list_memories(self, project_id: str):
        return self._manager.list_memories(project_id)

    async def save_memory(self, project_id: str, filename: str, content: str, mem_type: str = "project"):
        ok = self._manager.save_memory(project_id, filename, content, mem_type)
        return {"ok": ok}

    async def delete_memory(self, project_id: str, filename: str):
        ok = self._manager.delete_memory(project_id, filename)
        return {"ok": ok}

    async def search_memories(self, project_id: str, keyword: str):
        return self._manager.search_memories(project_id, keyword)

    async def get_memory_stats(self, project_id: str):
        return self._manager.get_memory_stats(project_id)

    async def get_relevant_memories(self, project_id: str, query: str):
        return self._manager.get_relevant_memories(project_id, query)

    async def list_templates(self, category: str = "all"):
        builtin = [
            {"id": "react-app", "name": "React 应用", "category": "frontend", "custom": False},
            {"id": "vue-app", "name": "Vue 应用", "category": "frontend", "custom": False},
            {"id": "flask-api", "name": "Flask API", "category": "backend", "custom": False},
            {"id": "fastapi-app", "name": "FastAPI 应用", "category": "backend", "custom": False},
            {"id": "static-site", "name": "静态网站", "category": "frontend", "custom": False},
            {"id": "landing-page", "name": "落地页", "category": "frontend", "custom": False},
        ]
        custom = self._manager.list_custom_templates()
        all_templates = builtin + custom
        if category and category != "all":
            all_templates = [t for t in all_templates if t.get("category") == category]
        return all_templates

    async def save_custom_template(self, name: str, category: str, desc: str, prompt: str, files: str = ""):
        ok = self._manager.save_custom_template(name, category, desc, prompt, files)
        return {"ok": ok}

    async def delete_custom_template(self, template_id: str):
        ok = self._manager.delete_custom_template(template_id)
        return {"ok": ok}

    async def create_from_template(self, project_id: str, template_id: str):
        return self._manager.create_project_from_template(project_id, template_id)
