import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ThemeProvider } from "./context/ThemeContext";
import Navbar from "./components/Navbar";
import Hero from "./components/Hero";
import ProductMatrix from "./components/ProductMatrix";
import ProductDetailPage from "./components/ProductDetailPage";
import PriceComparison from "./components/PriceComparison";
import AccountInfo from "./components/AccountInfo";
import DownloadSection from "./components/DownloadSection";
import Footer from "./components/Footer";

function AppContent() {
  const { t } = useTranslation();
  const [selectedProduct, setSelectedProduct] = useState<string | null>(null);

  useEffect(() => {
    document.title = t("app.title");
  }, [t]);

  if (selectedProduct) {
    return (
      <div className="min-h-screen bg-[var(--color-bg)] text-[var(--color-text)] transition-colors duration-300">
        <Navbar onLogoClick={() => setSelectedProduct(null)} />
        <ProductDetailPage productId={selectedProduct} />
        <Footer />
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[var(--color-bg)] text-[var(--color-text)] transition-colors duration-300">
      <Navbar />
      <main>
        <Hero />
        <ProductMatrix onSelectProduct={setSelectedProduct} />
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
