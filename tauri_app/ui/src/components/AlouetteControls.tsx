import React from "react";
import { Check, Minus, Plus } from "lucide-react";
import "./AlouetteControls.css";

export interface AlouetteNumberInputProps {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
  placeholder?: string;
  disabled?: boolean;
  size?: "sm" | "md";
  className?: string;
  style?: React.CSSProperties;
}

export function AlouetteNumberInput({
  value,
  onChange,
  min,
  max,
  step = 1,
  unit,
  placeholder,
  disabled = false,
  size = "md",
  className = "",
  style,
}: AlouetteNumberInputProps) {
  const clamp = (n: number) => {
    let next = n;
    if (typeof min === "number") next = Math.max(min, next);
    if (typeof max === "number") next = Math.min(max, next);
    return next;
  };

  const adjust = (delta: number) => {
    if (disabled) return;
    const base = Number.isFinite(value) ? value : min ?? 0;
    onChange(clamp(base + delta));
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const raw = e.target.value;
    if (raw === "") {
      onChange(min ?? 0);
      return;
    }
    const parsed = Number(raw);
    if (!Number.isNaN(parsed)) {
      onChange(parsed);
    }
  };

  const handleBlur = () => {
    if (!Number.isFinite(value)) {
      onChange(min ?? 0);
    } else {
      onChange(clamp(value));
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "ArrowUp") {
      e.preventDefault();
      adjust(step);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      adjust(-step);
    }
  };

  const atMin = typeof min === "number" && value <= min;
  const atMax = typeof max === "number" && value >= max;

  return (
    <div className={`alouette-num-wrap ${size} ${disabled ? "disabled" : ""} ${className}`} style={style}>
      <input
        type="number"
        value={Number.isFinite(value) ? value : ""}
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        placeholder={placeholder}
        onChange={handleInputChange}
        onBlur={handleBlur}
        onKeyDown={handleKeyDown}
        className="alouette-num-input"
      />
      {unit && <span className="alouette-num-unit">{unit}</span>}
      <div className="alouette-num-stepper">
        <button
          type="button"
          tabIndex={-1}
          disabled={disabled || atMin}
          onClick={() => adjust(-step)}
          className="alouette-num-btn"
          aria-label="Decrease"
        >
          <Minus size={11} />
        </button>
        <button
          type="button"
          tabIndex={-1}
          disabled={disabled || atMax}
          onClick={() => adjust(step)}
          className="alouette-num-btn"
          aria-label="Increase"
        >
          <Plus size={11} />
        </button>
      </div>
    </div>
  );
}

export interface AlouetteCheckboxProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: React.ReactNode;
  disabled?: boolean;
  className?: string;
}

export function AlouetteCheckbox({
  checked,
  onChange,
  label,
  disabled = false,
  className = "",
}: AlouetteCheckboxProps) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => !disabled && onChange(!checked)}
      className={`alouette-checkbox ${checked ? "checked" : ""} ${disabled ? "disabled" : ""} ${className}`}
    >
      <span className="alouette-checkbox-box">
        {checked && <Check size={11} strokeWidth={3} />}
      </span>
      {label !== undefined && label !== null && <span>{label}</span>}
    </button>
  );
}

export interface RadioOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface AlouetteRadioGroupProps {
  value: string;
  options: RadioOption[];
  onChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
}

export function AlouetteRadioGroup({
  value,
  options,
  onChange,
  disabled = false,
  className = "",
}: AlouetteRadioGroupProps) {
  return (
    <div role="radiogroup" className={`alouette-radio-group ${className}`}>
      {options.map((opt) => {
        const isChecked = opt.value === value;
        const isDisabled = disabled || opt.disabled;
        return (
          <button
            key={opt.value}
            type="button"
            role="radio"
            aria-checked={isChecked}
            disabled={isDisabled}
            onClick={() => !isDisabled && onChange(opt.value)}
            className={`alouette-radio-item ${isChecked ? "checked" : ""} ${isDisabled ? "disabled" : ""}`}
          >
            <span className="alouette-radio-circle">
              <span className="alouette-radio-dot" />
            </span>
            <span>{opt.label}</span>
          </button>
        );
      })}
    </div>
  );
}
