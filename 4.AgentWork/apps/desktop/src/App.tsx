import { Routes, Route, Navigate } from "react-router-dom";
import { ThemeProvider } from "@/lib/theme-store";
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
import Marketplace from "./views/Marketplace";
import TemplateSubmitWizard from "./views/TemplateSubmitWizard";
import TemplateCustomizeWizard from "./views/TemplateCustomizeWizard";
import ModelService from "./views/ModelService";
import DeliveryPreview, { MOCK_DELIVERY } from "./components/DeliveryPreview";

export default function App() {
  return (
    <ThemeProvider>
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
          <Route path="marketplace" element={<Marketplace />} />
          <Route path="marketplace/submit" element={<TemplateSubmitWizard />} />
          <Route path="marketplace/customize" element={<TemplateCustomizeWizard />} />
          <Route path="modelservice" element={<ModelService />} />
          <Route
            path="delivery"
            element={
              <DeliveryPreview {...MOCK_DELIVERY} />
            }
          />
          <Route path="settings" element={<Settings />} />
        </Route>
      </Routes>
    </ThemeProvider>
  );
}
