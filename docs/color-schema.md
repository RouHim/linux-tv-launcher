# 🦈 Rhinco-TV: UI Design System

Dieses Dokument beschreibt das Farbschema und die visuellen Richtlinien für den **Rhinco-TV** Launcher. Das Design basiert auf dem App-Icon (Walhai) und folgt einem **„Deep Ocean“**-Thema, das speziell für TV-Oberflächen (Dark Mode) optimiert ist.

## 🎨 Farbpalette

Das Farbschema ist in drei Ebenen unterteilt: **Tiefe (Hintergrund)**, **Ozean (Marke)** und **Biolumineszenz (Fokus/Akzente)**.

### 1. Hintergrund & Oberflächen (The Deep)
Diese Farben bilden die Basis. Sie sind dunkel genug, um Filmposter und Cover-Art wirken zu lassen, vermeiden aber „reines Schwarz“ (#000000), um einen Premium-Look zu gewährleisten.

| Name | Hex-Code | Verwendung |
| :--- | :--- | :--- |
| **Abyss Dark** | `#0B1016` | Haupt-Hintergrund der App. Ein sehr tiefes Blau-Schwarz. |
| **Deep Slate** | `#162231` | Hintergrund für Kacheln (Cards), Menüleisten oder Modals. |

### 2. Branding (The Ocean)
Abgeleitet vom Körper des Walhais. Diese Farben definieren die Identität der App.

| Name | Hex-Code | Verwendung |
| :--- | :--- | :--- |
| **Rhinco Blue** | `#1E5F94` | Header-Elemente, primäre Buttons, Branding-Flächen. |
| **Ocean Depth** | `#0F3555` | Wird als Endpunkt für Verläufe mit *Rhinco Blue* genutzt. |

### 3. Akzente & Fokus (Bioluminescence)
Für TV-Apps ist der **Focus State** essenziell. Diese Farben basieren auf den hellen Punkten des Walhais und müssen auf dunklem Grund „leuchten“.

| Name | Hex-Code | Verwendung |
| :--- | :--- | :--- |
| **Cyan Glow** | `#4CC9F0` | **Wichtig:** Fokus-Rahmen (Border), Fortschrittsbalken, aktive Icons. |
| **Soft White** | `#F0F4F8` | Hauptüberschriften. (Kein #FFFFFF nutzen, um Augenbelastung zu vermeiden). |
| **Muted Steel**| `#94A3B8` | Sekundärtext (Laufzeit, Jahr, Beschreibung), inaktive Menüpunkte. |

---

## 🛠 Implementation Reference (Design Tokens)

These definitions serve as the source of truth for the application styling.

### Base Colors (Primitive)

| Token | Hex | RGB (Float) | Description |
| :--- | :--- | :--- | :--- |
| `color_abyss_dark` | `#0B1016` | `0.04, 0.06, 0.09` | Deepest background |
| `color_deep_slate` | `#162231` | `0.09, 0.13, 0.19` | Surface color |
| `color_rhinco_blue`| `#1E5F94` | `0.12, 0.37, 0.58` | Brand primary |
| `color_ocean_depth`| `#0F3555` | `0.06, 0.21, 0.33` | Brand secondary |
| `color_cyan_glow`  | `#4CC9F0` | `0.30, 0.79, 0.94` | Accent / Focus |
| `color_soft_white` | `#F0F4F8` | `0.94, 0.96, 0.97` | Primary Text |
| `color_muted_steel`| `#94A3B8` | `0.58, 0.64, 0.72` | Secondary Text |

### Semantic Mapping

Use these semantic keys in the application code.

- **Backgrounds**
  - `bg_main` -> `color_abyss_dark`
  - `bg_surface` -> `color_deep_slate`

- **Brand**
  - `brand_primary` -> `color_rhinco_blue`
  - `brand_secondary` -> `color_ocean_depth`

- **Interaction**
  - `accent_focus` -> `color_cyan_glow`
  - `accent_active` -> `color_cyan_glow` (with opacity/glow)

- **Content**
  - `text_primary` -> `color_soft_white`
  - `text_secondary` -> `color_muted_steel`

- **Status / Feedback**
  - `status_success` -> `color_battery_good` (Green)
  - `status_warning` -> `color_battery_moderate` (Orange)
  - `status_error` -> `color_battery_low` (Red)

