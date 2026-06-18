import { Routes, Route, Navigate } from "react-router-dom";
import Layout from "./components/Layout";
import Timeline from "./views/Timeline";
import SnapshotDetail from "./views/SnapshotDetail";
import Branches from "./views/Branches";
import SemanticSearch from "./views/SemanticSearch";
import Team from "./views/Team";
import TaskBoard from "./views/TaskBoard";
import Settings from "./views/Settings";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<Navigate to="/timeline" replace />} />
        <Route path="timeline" element={<Timeline />} />
        <Route path="snapshot/:id" element={<SnapshotDetail />} />
        <Route path="branches" element={<Branches />} />
        <Route path="search" element={<SemanticSearch />} />
        <Route path="team" element={<Team />} />
        <Route path="tasks" element={<TaskBoard />} />
        <Route path="settings" element={<Settings />} />
      </Route>
    </Routes>
  );
}
