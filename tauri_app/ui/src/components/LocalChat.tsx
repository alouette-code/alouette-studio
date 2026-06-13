import React, { useState, useEffect, useRef } from "react";
import { ArrowLeft, Send, Cpu, Bot, User, Loader2 } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";

interface Message {
  role: "user" | "assistant";
  content: string;
}

interface LocalChatProps {
  onBack: () => void;
  initialMessage?: string;
  onClearInitialMessage?: () => void;
}

export default function LocalChat({
  onBack,
  initialMessage,
  onClearInitialMessage,
}: LocalChatProps) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputVal, setInputVal] = useState("");
  const [isTyping, setIsTyping] = useState(false);
  const [status, setStatus] = useState<"idle" | "starting" | "running">("idle");
  const [statusMessage, setStatusMessage] = useState("Sẵn sàng");
  
  const chatEndRef = useRef<HTMLDivElement>(null);
  const activeStreamContentRef = useRef<string>("");

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, isTyping]);

  // Handle incoming stream chunks
  useEffect(() => {
    let unlistenChunk: any;
    let unlistenStatus: any;
    let unlistenComplete: any;

    const setupListeners = async () => {
      unlistenChunk = await listen<string>("local-chat-chunk", (event) => {
        const chunk = event.payload;
        activeStreamContentRef.current += chunk;

        setMessages((prev) => {
          if (prev.length === 0) return prev;
          const last = prev[prev.length - 1];
          if (last.role === "assistant") {
            const next = [...prev];
            next[next.length - 1] = {
              ...last,
              content: activeStreamContentRef.current,
            };
            return next;
          }
          return prev;
        });
      });

      unlistenStatus = await listen<string>("local-chat-status", (event) => {
        const statusVal = event.payload as "starting" | "running";
        setStatus(statusVal);
        if (statusVal === "starting") {
          setStatusMessage("Đang khởi động local server...");
        } else {
          setStatusMessage("Mô hình local đang chạy");
        }
      });

      unlistenComplete = await listen<string>("local-chat-complete", () => {
        setIsTyping(false);
        activeStreamContentRef.current = "";
      });
    };

    setupListeners();

    return () => {
      if (unlistenChunk) unlistenChunk();
      if (unlistenStatus) unlistenStatus();
      if (unlistenComplete) unlistenComplete();
    };
  }, []);

  // Send message handler
  const handleSend = async (textToSend: string) => {
    if (!textToSend.trim() || isTyping) return;

    const userMsg: Message = { role: "user", content: textToSend.trim() };
    const historyForBackend = [...messages];
    
    // Add user message to state
    setMessages((prev) => [...prev, userMsg]);
    setInputVal("");
    setIsTyping(true);

    // Prepare placeholder for streaming response
    activeStreamContentRef.current = "";
    setMessages((prev) => [...prev, { role: "assistant", content: "" }]);

    try {
      await invoke("local_chat_send", {
        message: userMsg.content,
        history: historyForBackend,
      });
    } catch (err: any) {
      console.error(err);
      setStatusMessage("Lỗi kết nối server");
      setMessages((prev) => {
        const next = [...prev];
        if (next.length > 0 && next[next.length - 1].role === "assistant") {
          next[next.length - 1] = {
            role: "assistant",
            content: `❌ Lỗi: ${err?.message || err || "Không thể kết nối đến local server. Hãy đảm bảo bạn đã cài đặt llama.cpp."}`,
          };
        }
        return next;
      });
      setIsTyping(false);
    }
  };

  // Handle initial message passed from Welcome Page
  useEffect(() => {
    if (initialMessage && initialMessage.trim()) {
      handleSend(initialMessage);
      if (onClearInitialMessage) {
        onClearInitialMessage();
      }
    }
  }, [initialMessage]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend(inputVal);
    }
  };

  return (
    <div className="local-chat-container">
      {/* HEADER */}
      <div className="local-chat-header">
        <button onClick={onBack} className="back-btn" title="Quay lại">
          <ArrowLeft size={16} />
        </button>
        <div className="title-section">
          <h3>Trợ lý Local Phi-3</h3>
          <div className="status-indicator">
            <span className={`status-dot ${status}`}></span>
            <span className="status-text">{statusMessage}</span>
          </div>
        </div>
      </div>

      {/* CHAT MESSAGES PANEL */}
      <div className="local-chat-messages">
        {messages.length === 0 ? (
          <div className="chat-empty-state">
            <Cpu className="empty-icon animate-pulse" size={48} />
            <h4>Bắt đầu trò chuyện với Phi-3</h4>
            <p>Mô hình local chạy trực tiếp trên thiết bị của bạn bằng llama.cpp.</p>
          </div>
        ) : (
          messages.map((msg, index) => (
            <div key={index} className={`chat-message-wrapper ${msg.role}`}>
              <div className="avatar">
                {msg.role === "assistant" ? <Bot size={14} /> : <User size={14} />}
              </div>
              <div className="message-bubble">
                {msg.role === "assistant" && msg.content === "" ? (
                  <div className="thinking-dots">
                    <Loader2 className="animate-spin" size={16} />
                    <span>Đang suy nghĩ...</span>
                  </div>
                ) : (
                  <ReactMarkdown>{msg.content}</ReactMarkdown>
                )}
              </div>
            </div>
          ))
        )}
        <div ref={chatEndRef} />
      </div>

      {/* INPUT PANEL */}
      <div className="local-chat-input-area">
        <div className="input-wrapper">
          <textarea
            value={inputVal}
            onChange={(e) => setInputVal(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Nhập câu hỏi tại đây..."
            rows={1}
            disabled={isTyping}
          />
          <button
            onClick={() => handleSend(inputVal)}
            disabled={!inputVal.trim() || isTyping}
            className="send-btn"
          >
            <Send size={16} />
          </button>
        </div>
      </div>
    </div>
  );
}
