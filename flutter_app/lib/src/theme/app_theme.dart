import 'package:flutter/material.dart';

class AppColors {
  // Dark theme
  static const darkBg = Color(0xFF111118);
  static const darkSurface = Color(0xFF1A1A24);
  static const darkBorder = Color(0xFF2A2A3A);
  static const darkTextPrimary = Color(0xFFE8E8EF);
  static const darkTextSecondary = Color(0xFF8888A0);
  static const darkAccent = Color(0xFF6366F1);
  static const darkAccentPurple = Color(0xFFA78BFA);
  static const darkDanger = Color(0xFFEF4444);
  static const darkWarning = Color(0xFFF59E0B);
  static const darkSuccess = Color(0xFF22C55E);
  static const darkInfo = Color(0xFF3B82F6);
  static const darkHover = Color(0xFF2A2A3E);

  // Light theme
  static const lightBg = Color(0xFFF5F5F8);
  static const lightSurface = Color(0xFFFFFFFF);
  static const lightBorder = Color(0xFFE0E0E8);
  static const lightTextPrimary = Color(0xFF1A1A24);
  static const lightTextSecondary = Color(0xFF6B6B80);
  static const lightAccent = Color(0xFF6366F1);
  static const lightHover = Color(0xFFEEEEF4);
}

class AppTheme {
  static ThemeData darkTheme() {
    return ThemeData(
      brightness: Brightness.dark,
      scaffoldBackgroundColor: AppColors.darkBg,
      colorScheme: const ColorScheme.dark(
        primary: AppColors.darkAccent,
        secondary: AppColors.darkAccentPurple,
        surface: AppColors.darkSurface,
        error: AppColors.darkDanger,
      ),
      dividerColor: AppColors.darkBorder,
      textTheme: const TextTheme(
        bodySmall: TextStyle(color: AppColors.darkTextSecondary, fontSize: 11),
        bodyMedium: TextStyle(color: AppColors.darkTextPrimary, fontSize: 13),
        bodyLarge: TextStyle(color: AppColors.darkTextPrimary, fontSize: 14),
        titleSmall: TextStyle(color: AppColors.darkTextPrimary, fontSize: 12, fontWeight: FontWeight.w600),
        titleMedium: TextStyle(color: AppColors.darkTextPrimary, fontSize: 14, fontWeight: FontWeight.w600),
        titleLarge: TextStyle(color: AppColors.darkTextPrimary, fontSize: 16, fontWeight: FontWeight.w600),
      ),
    );
  }

  static ThemeData lightTheme() {
    return ThemeData(
      brightness: Brightness.light,
      scaffoldBackgroundColor: AppColors.lightBg,
      colorScheme: const ColorScheme.light(
        primary: AppColors.lightAccent,
        surface: AppColors.lightSurface,
        error: AppColors.darkDanger,
      ),
      dividerColor: AppColors.lightBorder,
      textTheme: const TextTheme(
        bodySmall: TextStyle(color: AppColors.lightTextSecondary, fontSize: 11),
        bodyMedium: TextStyle(color: AppColors.lightTextPrimary, fontSize: 13),
        bodyLarge: TextStyle(color: AppColors.lightTextPrimary, fontSize: 14),
        titleSmall: TextStyle(color: AppColors.lightTextPrimary, fontSize: 12, fontWeight: FontWeight.w600),
        titleMedium: TextStyle(color: AppColors.lightTextPrimary, fontSize: 14, fontWeight: FontWeight.w600),
        titleLarge: TextStyle(color: AppColors.lightTextPrimary, fontSize: 16, fontWeight: FontWeight.w600),
      ),
    );
  }

  static Color borderPrimary(BuildContext context) {
    return Theme.of(context).brightness == Brightness.dark
        ? AppColors.darkBorder
        : AppColors.lightBorder;
  }

  static Color textPrimary(BuildContext context) {
    return Theme.of(context).brightness == Brightness.dark
        ? AppColors.darkTextPrimary
        : AppColors.lightTextPrimary;
  }

  static Color textSecondary(BuildContext context) {
    return Theme.of(context).brightness == Brightness.dark
        ? AppColors.darkTextSecondary
        : AppColors.lightTextSecondary;
  }

  static Color hoverColor(BuildContext context) {
    return Theme.of(context).brightness == Brightness.dark
        ? AppColors.darkHover
        : AppColors.lightHover;
  }

  static Color accentColor(BuildContext context) {
    return Theme.of(context).brightness == Brightness.dark
        ? AppColors.darkAccent
        : AppColors.lightAccent;
  }
}
