import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ThemeProvider } from "./context/ThemeContext";
import Navbar from "./components/Navbar";
import Hero from "./components/Hero";
import ProductShowcase from "./components/ProductShowcase";
import Features from "./components/Features";
import Templates from "./components/Templates";
import Architecture from "./components/Architecture";
import Pricing from "./components/Pricing";
import CtaSection from "./components/CtaSection";
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
        <ProductShowcase />
        <Features />
        <Templates />
        <Architecture />
        <Pricing />
        <CtaSection />
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
