import { Routes, Route, Navigate } from "react-router-dom";
import { ThemeProvider } from "@/lib/theme-store";
import { I18nProvider } from "@/i18n";
import Layout from "./components/Layout";
import Onboarding, { isOnboarded } from "./views/Onboarding";
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
import EvolutionDashboard from "./views/EvolutionDashboard";
import ToolBuilder from "./views/ToolBuilder";
import DeliveryPreview, { MOCK_DELIVERY } from "./components/DeliveryPreview";

/** 首次启动重定向：未完成向导则跳转 /onboarding */
function OnboardingGuard({ children }: { children: React.ReactNode }) {
  if (!isOnboarded()) {
    return <Navigate to="/onboarding" replace />;
  }
  return <>{children}</>;
}

export default function App() {
  return (
    <I18nProvider>
      <ThemeProvider>
        <Routes>
          {/* 向导页不套 Layout，全屏展示 */}
          <Route path="/onboarding" element={<Onboarding />} />
          <Route
            path="/"
            element={
              <OnboardingGuard>
                <Layout />
              </OnboardingGuard>
            }
          >
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
            <Route path="evolution" element={<EvolutionDashboard />} />
            <Route path="toolbuilder" element={<ToolBuilder />} />
            <Route
              path="delivery"
              element={
                <DeliveryPreview {...MOCK_DELIVERY} />
              }
            />
            <Route path="settings" element={<Settings />} />
          </Route>
          {/* 兜底：未知路径回到首页（会触发向导检测） */}
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </ThemeProvider>
    </I18nProvider>
  );
}