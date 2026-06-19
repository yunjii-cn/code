import { Routes, Route, Navigate } from "react-router-dom";
import Layout from "./components/Layout";
import Timeline from "./views/Timeline";
import SnapshotDetail from "./views/SnapshotDetail";
import Branches from "./views/Branches";
import SemanticSearch from "./views/SemanticSearch";
import Team from "./views/Team";
import TaskBoard from "./views/TaskBoard";
import ChatPanel from "./views/ChatPanel";
import Settings from "./views/Settings";
import TrainingWizard from "./views/TrainingWizard";
import DeliveryPreview, { MOCK_DELIVERY } from "./components/DeliveryPreview";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<Navigate to="/chat" replace />} />
        <Route path="chat" element={<ChatPanel />} />
        <Route path="timeline" element={<Timeline />} />
        <Route path="snapshot/:id" element={<SnapshotDetail />} />
        <Route path="branches" element={<Branches />} />
        <Route path="search" element={<SemanticSearch />} />
        <Route path="team" element={<Team />} />
        <Route path="tasks" element={<TaskBoard />} />
        <Route path="training" element={<TrainingWizard />} />
        <Route
          path="delivery"
          element={
            <DeliveryPreview {...MOCK_DELIVERY} />
          }
        />
        <Route path="settings" element={<Settings />} />
      </Route>
    </Routes>
  );
}
