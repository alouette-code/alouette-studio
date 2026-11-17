import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown, Search } from "lucide-react";
import "./AlouetteSelect.css";

export interface SelectOption {
  value: string;
  label: string;
  group?: string;
  description?: string;
  badge?: string;
  color?: string;
  disabled?: boolean;
}

export interface AlouetteSelectProps {
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  searchable?: boolean;
  searchPlaceholder?: string;
  size?: "sm" | "md";
  maxDropdownHeight?: number;
  className?: string;
  style?: React.CSSProperties;
}

interface PopoverCoords {
  top: number;
  left: number;
  width: number;
  maxHeight: number;
}

export function AlouetteSelect({
  value,
  options,
  onChange,
  placeholder = "Select...",
  disabled = false,
  searchable,
  searchPlaceholder = "Search...",
  size = "md",
  maxDropdownHeight = 280,
  className = "",
  style,
}: AlouetteSelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [highlightedIdx, setHighlightedIdx] = useState(0);
  const [coords, setCoords] = useState<PopoverCoords | null>(null);

  const triggerRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const showSearch = searchable ?? options.length >= 10;

  const selectedOption = useMemo(
    () => options.find((opt) => opt.value === value) || null,
    [options, value]
  );

  const filteredOptions = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return options;
    return options.filter(
      (opt) =>
        opt.label.toLowerCase().includes(q) ||
        opt.value.toLowerCase().includes(q) ||
        (opt.group && opt.group.toLowerCase().includes(q)) ||
        (opt.description && opt.description.toLowerCase().includes(q))
    );
  }, [options, query]);

  // Group filtered options while preserving insertion order
  const groupedEntries = useMemo(() => {
    const sections: { group: string | null; items: { option: SelectOption; flatIndex: number }[] }[] = [];
    const groupMap = new Map<string, number>();

    filteredOptions.forEach((option, flatIndex) => {
      const gKey = option.group ?? "";
      if (!option.group) {
        if (sections.length === 0 || sections[sections.length - 1].group !== null) {
          sections.push({ group: null, items: [] });
        }
        sections[sections.length - 1].items.push({ option, flatIndex });
      } else if (groupMap.has(gKey)) {
        sections[groupMap.get(gKey)!].items.push({ option, flatIndex });
      } else {
        groupMap.set(gKey, sections.length);
        sections.push({ group: option.group, items: [{ option, flatIndex }] });
      }
    });

    return sections;
  }, [filteredOptions]);

  const updatePosition = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const margin = 8;
    const spaceBelow = window.innerHeight - rect.bottom - margin;
    const spaceAbove = rect.top - margin;
    const searchExtra = showSearch ? 36 : 0;
    const estimatedHeight = Math.min(maxDropdownHeight, filteredOptions.length * 32 + 12 + searchExtra);

    const openUpward = spaceBelow < Math.min(estimatedHeight, 180) && spaceAbove > spaceBelow;
    const available = Math.max(120, openUpward ? spaceAbove : spaceBelow);
    const maxHeight = Math.min(maxDropdownHeight + searchExtra, available);

    const top = openUpward
      ? Math.max(margin, rect.top - Math.min(estimatedHeight, maxHeight) - 2)
      : rect.bottom + 2;

    const minWidth = Math.max(rect.width, 180);
    const left = Math.min(rect.left, window.innerWidth - minWidth - margin);

    setCoords({
      top,
      left: Math.max(margin, left),
      width: rect.width,
      maxHeight,
    });
  }, [filteredOptions.length, maxDropdownHeight, showSearch]);

  useLayoutEffect(() => {
    if (!isOpen) return;
    updatePosition();
  }, [isOpen, updatePosition]);

  useEffect(() => {
    if (!isOpen) return;

    const currentIdx = filteredOptions.findIndex((o) => o.value === value && !o.disabled);
    setHighlightedIdx(currentIdx >= 0 ? currentIdx : 0);

    if (showSearch) {
      setTimeout(() => searchInputRef.current?.focus(), 0);
    }
  }, [isOpen, showSearch]);

  useEffect(() => {
    if (!isOpen) return;

    const handleOutsideClick = (e: MouseEvent) => {
      const target = e.target as Node;
      if (
        triggerRef.current?.contains(target) ||
        popoverRef.current?.contains(target)
      ) {
        return;
      }
      setIsOpen(false);
    };

    const handleWindowChange = () => updatePosition();

    document.addEventListener("mousedown", handleOutsideClick);
    window.addEventListener("resize", handleWindowChange);
    window.addEventListener("scroll", handleWindowChange, true);
    return () => {
      document.removeEventListener("mousedown", handleOutsideClick);
      window.removeEventListener("resize", handleWindowChange);
      window.removeEventListener("scroll", handleWindowChange, true);
    };
  }, [isOpen, updatePosition]);

  const closeMenu = () => {
    setIsOpen(false);
    setQuery("");
    triggerRef.current?.focus();
  };

  const selectOption = (opt: SelectOption) => {
    if (opt.disabled) return;
    onChange(opt.value);
    closeMenu();
  };

  const moveHighlight = (delta: number) => {
    if (filteredOptions.length === 0) return;
    let next = highlightedIdx;
    for (let step = 0; step < filteredOptions.length; step++) {
      next = (next + delta + filteredOptions.length) % filteredOptions.length;
      if (!filteredOptions[next].disabled) {
        setHighlightedIdx(next);
        const el = listRef.current?.querySelector(`[data-option-idx="${next}"]`) as HTMLElement | null;
        el?.scrollIntoView({ block: "nearest" });
        break;
      }
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (disabled) return;

    if (!isOpen) {
      if (e.key === "Enter" || e.key === " " || e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        setIsOpen(true);
      }
      return;
    }

    switch (e.key) {
      case "Escape":
        e.preventDefault();
        closeMenu();
        break;
      case "ArrowDown":
        e.preventDefault();
        moveHighlight(1);
        break;
      case "ArrowUp":
        e.preventDefault();
        moveHighlight(-1);
        break;
      case "Enter":
        e.preventDefault();
        if (filteredOptions[highlightedIdx]) {
          selectOption(filteredOptions[highlightedIdx]);
        }
        break;
    }
  };

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        onClick={() => !disabled && setIsOpen((prev) => !prev)}
        onKeyDown={handleKeyDown}
        className={`alouette-select-trigger ${size} ${isOpen ? "open" : ""} ${className}`}
        style={style}
      >
        <span
          className={`alouette-select-value ${!selectedOption ? "alouette-select-placeholder" : ""}`}
          style={selectedOption?.color ? { color: selectedOption.color, fontWeight: 700 } : undefined}
        >
          {selectedOption ? selectedOption.label : placeholder}
        </span>
        <ChevronDown size={13} className="alouette-select-chevron" />
      </button>

      {isOpen &&
        coords &&
        createPortal(
          <div
            ref={popoverRef}
            role="listbox"
            className="alouette-select-popover"
            onKeyDown={handleKeyDown}
            style={{
              top: `${coords.top}px`,
              left: `${coords.left}px`,
              minWidth: `${Math.max(coords.width, 180)}px`,
              width: `${coords.width}px`,
              maxHeight: `${coords.maxHeight}px`,
            }}
          >
            {showSearch && (
              <div className="alouette-select-search-wrap">
                <Search size={12} color="var(--text-muted)" />
                <input
                  ref={searchInputRef}
                  type="text"
                  value={query}
                  onChange={(e) => {
                    setQuery(e.target.value);
                    setHighlightedIdx(0);
                  }}
                  placeholder={searchPlaceholder}
                  className="alouette-select-search-input"
                />
              </div>
            )}

            <div ref={listRef} className="alouette-select-list">
              {groupedEntries.map((section, sIdx) => (
                <div key={section.group ?? `nogroup-${sIdx}`}>
                  {section.group && <div className="alouette-select-group">{section.group}</div>}
                  {section.items.map(({ option, flatIndex }) => {
                    const isSelected = option.value === value;
                    const isHighlighted = flatIndex === highlightedIdx;
                    return (
                      <div
                        key={option.value}
                        role="option"
                        aria-selected={isSelected}
                        data-option-idx={flatIndex}
                        onMouseEnter={() => setHighlightedIdx(flatIndex)}
                        onMouseDown={(e) => {
                          e.preventDefault();
                          selectOption(option);
                        }}
                        className={`alouette-select-option ${isSelected ? "selected" : ""} ${
                          isHighlighted ? "highlighted" : ""
                        } ${option.disabled ? "disabled" : ""}`}
                      >
                        <div className="alouette-select-option-main">
                          <span
                            className="alouette-select-option-label"
                            style={option.color ? { color: option.color, fontWeight: 700 } : undefined}
                          >
                            {option.label}
                          </span>
                          {option.description && (
                            <span className="alouette-select-option-desc">{option.description}</span>
                          )}
                        </div>
                        {option.badge && (
                          <span className="alouette-select-option-badge">{option.badge}</span>
                        )}
                        <span className="alouette-select-check-slot">
                          {isSelected && <Check size={12} color="var(--color-accent)" />}
                        </span>
                      </div>
                    );
                  })}
                </div>
              ))}

              {filteredOptions.length === 0 && (
                <div className="alouette-select-empty">No matching options</div>
              )}
            </div>
          </div>,
          document.body
        )}
    </>
  );
}

export default AlouetteSelect;
