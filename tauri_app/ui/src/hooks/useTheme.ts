import { useState, useEffect } from "react";

export function useTheme() {
  const [theme, setThemeState] = useState<"dark" | "light">(() => {
    const savedTheme = localStorage.getItem("alouette_theme");
    return savedTheme === "dark" || savedTheme === "light" ? savedTheme : "dark";
  });

  const setTheme = (newTheme: "dark" | "light") => {
    setThemeState(newTheme);
    localStorage.setItem("alouette_theme", newTheme);
    document.documentElement.setAttribute("data-theme", newTheme);
    window.dispatchEvent(new Event("alouette-theme-change"));
  };

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, []);

  useEffect(() => {
    const handleStorage = (e: StorageEvent) => {
      if (e.key === "alouette_theme" && e.newValue) {
        const newTheme = e.newValue as "dark" | "light";
        setThemeState(newTheme);
        document.documentElement.setAttribute("data-theme", newTheme);
      }
    };

    const handleCustom = () => {
      const savedTheme = localStorage.getItem("alouette_theme");
      if (savedTheme === "dark" || savedTheme === "light") {
        setThemeState(savedTheme);
        document.documentElement.setAttribute("data-theme", savedTheme);
      }
    };

    window.addEventListener("storage", handleStorage);
    window.addEventListener("alouette-theme-change", handleCustom);
    return () => {
      window.removeEventListener("storage", handleStorage);
      window.removeEventListener("alouette-theme-change", handleCustom);
    };
  }, []);

  return { theme, setTheme };
}
