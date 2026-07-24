import { ReactNode } from "react";
import { getIcon } from "material-file-icons";

interface FileDevIconProps {
  fileName: string;
  size?: number;
  fallbackIcon?: ReactNode;
}

const MATERIAL_SYMBOLS_HTML = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -960 960 960" width="100%" height="100%" fill="#e65100"><path d="M320-240 80-480l240-240 57 57-183 183 183 183-57 57Zm320 0-57-57 183-183-183-183 57-57 240 240-240 240Z"/></svg>`;

export function FileDevIcon({ fileName, size = 14, fallbackIcon }: FileDevIconProps) {
  const nameOnly = fileName ? (fileName.split(/[\\/]/).pop() || fileName).toLowerCase() : "";

  if (nameOnly.endsWith(".html") || nameOnly.endsWith(".htm")) {
    return (
      <span
        className="file-dev-icon"
        style={{
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
          width: size,
          height: size,
          marginRight: 5,
          flexShrink: 0,
          verticalAlign: "middle",
        }}
        dangerouslySetInnerHTML={{ __html: MATERIAL_SYMBOLS_HTML }}
      />
    );
  }

  if (nameOnly.endsWith(".css")) {
    return (
      <span
        className="file-dev-icon-text"
        style={{
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
          width: size,
          height: size,
          marginRight: 5,
          color: "#42a5f5",
          fontWeight: 700,
          fontSize: `${size + 1}px`,
          lineHeight: 1,
          fontFamily: "Consolas, 'Fira Code', monospace, sans-serif",
          flexShrink: 0,
          userSelect: "none",
          verticalAlign: "middle",
        }}
      >
        #
      </span>
    );
  }

  const icon = getIcon(nameOnly);
  if (!icon || !icon.svg) {
    return <>{fallbackIcon || null}</>;
  }

  return (
    <span
      className="file-dev-icon"
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: size,
        height: size,
        marginRight: 5,
        flexShrink: 0,
        verticalAlign: "middle",
      }}
      dangerouslySetInnerHTML={{ __html: icon.svg }}
    />
  );
}

export default FileDevIcon;
