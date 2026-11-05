import { Routes, Route, Navigate } from "react-router-dom";
import { Zap, Lightbulb, Workflow, Bot } from "lucide-react";
import { ThemeProvider } from "@/lib/theme-store";
import { I18nProvider } from "@/i18n";
import { useAuth } from "@/lib/auth-store";
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
import UpdateManager from "./views/UpdateManager";
import Login from "./views/Login";
import EcoPlaceholder from "./views/EcoPlaceholder";
import DeliveryPreview, { MOCK_DELIVERY } from "./components/DeliveryPreview";

/** 首次启动重定向：未完成向导则跳转 /onboarding */
function OnboardingGuard({ children }: { children: React.ReactNode }) {
  if (!isOnboarded()) {
    return <Navigate to="/onboarding" replace />;
  }
  return <>{children}</>;
}

/** 已认证应用：未登录则展示全屏品牌登录页 */
function AuthenticatedApp() {
  const { user } = useAuth();

  if (!user) {
    return <Login />;
  }

  return (
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
        <Route path="update" element={<UpdateManager />} />
        {/* 生态占位页 */}
        <Route
          path="skills"
          element={
            <EcoPlaceholder
              icon={Zap}
              titleKey="nav.skills"
              descKey="eco.skills.desc"
              features={[
                "eco.skills.f1",
                "eco.skills.f2",
                "eco.skills.f3",
              ]}
            />
          }
        />
        <Route
          path="inspiration"
          element={
            <EcoPlaceholder
              icon={Lightbulb}
              titleKey="nav.inspiration"
              descKey="eco.inspiration.desc"
              features={[
                "eco.inspiration.f1",
                "eco.inspiration.f2",
                "eco.inspiration.f3",
              ]}
            />
          }
        />
        <Route
          path="automation"
          element={
            <EcoPlaceholder
              icon={Workflow}
              titleKey="nav.automation"
              descKey="eco.automation.desc"
              features={[
                "eco.automation.f1",
                "eco.automation.f2",
                "eco.automation.f3",
              ]}
            />
          }
        />
        <Route
          path="agents"
          element={
            <EcoPlaceholder
              icon={Bot}
              titleKey="nav.agents"
              descKey="eco.agents.desc"
              features={[
                "eco.agents.f1",
                "eco.agents.f2",
                "eco.agents.f3",
              ]}
            />
          }
        />
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
  );
}

export default function App() {
  return (
    <I18nProvider>
      <ThemeProvider>
        <AuthenticatedApp />
      </ThemeProvider>
    </I18nProvider>
  );
}