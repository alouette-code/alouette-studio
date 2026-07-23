import { useState, useEffect } from "react";

export type EditorEngine = "monaco" | "codemirror";

export function useEditorEngine() {
  const [editorEngine, setEditorEngineState] = useState<EditorEngine>(() => {
    const saved = localStorage.getItem("editor_engine");
    return saved === "codemirror" ? "codemirror" : "monaco";
  });

  const setEditorEngine = (newEngine: EditorEngine) => {
    setEditorEngineState(newEngine);
    localStorage.setItem("editor_engine", newEngine);
    window.dispatchEvent(new Event("alouette-editor-engine-change"));
  };

  useEffect(() => {
    const handleStorage = (e: StorageEvent) => {
      if (e.key === "editor_engine" && e.newValue) {
        const newEngine = e.newValue === "codemirror" ? "codemirror" : "monaco";
        setEditorEngineState(newEngine);
      }
    };

    const handleCustom = () => {
      const saved = localStorage.getItem("editor_engine");
      const newEngine = saved === "codemirror" ? "codemirror" : "monaco";
      setEditorEngineState(newEngine);
    };

    window.addEventListener("storage", handleStorage);
    window.addEventListener("alouette-editor-engine-change", handleCustom);
    return () => {
      window.removeEventListener("storage", handleStorage);
      window.removeEventListener("alouette-editor-engine-change", handleCustom);
    };
  }, []);

  return { editorEngine, setEditorEngine };
}
