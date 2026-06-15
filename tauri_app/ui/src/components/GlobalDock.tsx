import {
  FolderPen,
  BrainCircuit,
  Droplets,
  Proportions,
  UserRoundKey
} from "lucide-react";

interface GlobalDockProps {
  onOpenLocalAi?: () => void;
}

export default function GlobalDock({ onOpenLocalAi }: GlobalDockProps) {
  return (
    <div className="global-dock">
      <div className="dock-top">
        <button className="dock-btn active" title="Explorer">
          <FolderPen size={20} />
        </button>
        <button className="dock-btn" title="AI Assistant" onClick={onOpenLocalAi}>
          <BrainCircuit size={20} />
        </button>
        <button className="dock-btn" title="Memory Leak">
          <Droplets size={20} />
        </button>
        <button className="dock-btn" title="VM (Virtual Machine)">
          <Proportions size={20} />
        </button>
      </div>
      <div className="dock-bottom">
        <button className="dock-btn" title="User Profile">
          <UserRoundKey size={20} />
        </button>
      </div>
    </div>
  );
}
