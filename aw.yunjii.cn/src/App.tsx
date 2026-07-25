import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ThemeProvider } from "./context/ThemeContext";
import Navbar from "./components/Navbar";
import Hero from "./components/Hero";
import ProductShowcase from "./components/ProductShowcase";
import Features from "./components/Features";
import Templates from "./components/Templates";
import Architecture from "./components/Architecture";
import Pricing from "./components/Pricing";
import Roadmap from "./components/Roadmap";
import CtaSection from "./components/CtaSection";
import DemoForm from "./components/DemoForm";
import Faq from "./components/Faq";
import Footer from "./components/Footer";

function AppContent() {
  const { t } = useTranslation();
  const [demoOpen, setDemoOpen] = useState(false);

  useEffect(() => {
    document.title = t("app.title");
  }, [t]);

  return (
    <div className="min-h-screen bg-[var(--color-bg)] text-[var(--color-text)] transition-colors duration-300">
      <Navbar onOpenDemo={() => setDemoOpen(true)} />
      <main>
        <Hero onOpenDemo={() => setDemoOpen(true)} />
        <ProductShowcase />
        <Features />
        <Templates />
        <Architecture />
        <Pricing onOpenDemo={() => setDemoOpen(true)} />
        <Roadmap />
        <CtaSection onOpenDemo={() => setDemoOpen(true)} />
        <Faq />
      </main>
      <Footer />
      <DemoForm isOpen={demoOpen} onClose={() => setDemoOpen(false)} />
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
