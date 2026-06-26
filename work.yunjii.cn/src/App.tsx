import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ThemeProvider } from "./context/ThemeContext";
import Navbar from "./components/Navbar";
import Hero from "./components/Hero";
import ProductMatrix from "./components/ProductMatrix";
import { DesktopDetail, WebDetail, TeamDetail } from "./components/ProductDetail";
import PriceComparison from "./components/PriceComparison";
import AccountInfo from "./components/AccountInfo";
import DownloadSection from "./components/DownloadSection";
import Footer from "./components/Footer";

function AppContent() {
  const { t } = useTranslation();

  useEffect(() => {
    document.title = t("app.title");
  }, [t]);

  return (
    <div className="min-h-screen bg-[var(--color-bg)] text-[var(--color-text)] transition-colors duration-300">
      <Navbar />
      <main>
        <Hero />
        <ProductMatrix />
        <DesktopDetail />
        <WebDetail />
        <TeamDetail />
        <PriceComparison />
        <AccountInfo />
        <DownloadSection />
      </main>
      <Footer />
    </div>
  );
}

export default function App() {
  return (
    <ThemeProvider>
      <AppContent />
    </ThemeProvider>
  );
}
