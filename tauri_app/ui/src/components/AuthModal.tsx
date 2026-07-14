import { useState } from "react";
import { X, UserRoundKey } from "lucide-react";

interface AuthModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function AuthModal({ isOpen, onClose }: AuthModalProps) {
  const [mode, setMode] = useState<"login" | "register">("login");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" style={{ zIndex: 1000, backdropFilter: "blur(4px)" }} onClick={onClose}>
      <div 
        className="modal-content" 
        onClick={(e) => e.stopPropagation()} 
        style={{ 
          padding: "0", 
          display: "flex", 
          flexDirection: "column", 
          background: "var(--bg-secondary)", 
          borderRadius: "12px", 
          border: "1px solid var(--border-primary)",
          width: "380px",
          boxShadow: "0 24px 48px rgba(0, 0, 0, 0.5)"
        }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "16px 20px", borderBottom: "1px solid var(--border-primary)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
             <UserRoundKey size={18} style={{ color: "#3a86ff" }}/>
             <h3 style={{ margin: 0, fontSize: "16px", fontWeight: "600", color: "var(--text-primary)" }}>
               {mode === "login" ? "Welcome Back" : "Create Account"}
             </h3>
          </div>
          <button className="icon-btn" onClick={onClose} style={{ background: "transparent", border: "none", cursor: "pointer", color: "var(--text-secondary)" }}>
            <X size={18} />
          </button>
        </div>

        <div style={{ padding: "24px 20px", display: "flex", flexDirection: "column", gap: "20px" }}>
          <div style={{ display: "flex", gap: "8px", background: "var(--bg-tertiary)", padding: "4px", borderRadius: "8px" }}>
             <button 
               style={{ 
                 flex: 1, 
                 padding: "8px 12px", 
                 background: mode === "login" ? "var(--bg-secondary)" : "transparent",
                 color: mode === "login" ? "var(--text-primary)" : "var(--text-secondary)",
                 border: mode === "login" ? "1px solid var(--border-primary)" : "1px solid transparent",
                 borderRadius: "6px",
                 cursor: "pointer",
                 fontWeight: 500
               }}
               onClick={() => setMode("login")}
             >
               Login
             </button>
             <button 
               style={{ 
                 flex: 1, 
                 padding: "8px 12px", 
                 background: mode === "register" ? "var(--bg-secondary)" : "transparent",
                 color: mode === "register" ? "var(--text-primary)" : "var(--text-secondary)",
                 border: mode === "register" ? "1px solid var(--border-primary)" : "1px solid transparent",
                 borderRadius: "6px",
                 cursor: "pointer",
                 fontWeight: 500
               }}
               onClick={() => setMode("register")}
             >
               Register
             </button>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
            <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
              <label style={{ fontSize: "13px", color: "var(--text-secondary)", fontWeight: 500 }}>Username</label>
              <input 
                type="text" 
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="Enter your username"
                style={{ 
                  width: "100%", 
                  boxSizing: "border-box",
                  background: "var(--bg-tertiary)",
                  border: "1px solid var(--border-primary)",
                  color: "var(--text-primary)",
                  padding: "10px 12px",
                  borderRadius: "8px",
                  outline: "none",
                  fontSize: "14px"
                }}
                onFocus={(e) => e.target.style.borderColor = "#3a86ff"}
                onBlur={(e) => e.target.style.borderColor = "var(--border-primary)"}
              />
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
              <label style={{ fontSize: "13px", color: "var(--text-secondary)", fontWeight: 500 }}>Password</label>
              <input 
                type="password" 
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Enter your password"
                style={{ 
                  width: "100%", 
                  boxSizing: "border-box",
                  background: "var(--bg-tertiary)",
                  border: "1px solid var(--border-primary)",
                  color: "var(--text-primary)",
                  padding: "10px 12px",
                  borderRadius: "8px",
                  outline: "none",
                  fontSize: "14px"
                }}
                onFocus={(e) => e.target.style.borderColor = "#3a86ff"}
                onBlur={(e) => e.target.style.borderColor = "var(--border-primary)"}
              />
            </div>
          </div>

          <button 
             style={{ 
               marginTop: "8px", 
               padding: "12px", 
               background: "#3a86ff", 
               color: "#fff", 
               border: "none", 
               borderRadius: "8px", 
               cursor: "pointer",
               fontWeight: 600,
               fontSize: "14px"
             }}
             onClick={() => {
                 alert(`${mode === "login" ? "Logging in" : "Registering"} with ${username}...`);
                 onClose();
             }}
          >
            {mode === "login" ? "Sign In" : "Create Account"}
          </button>
        </div>
      </div>
    </div>
  );
}
