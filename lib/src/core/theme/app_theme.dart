// UOT App Theme System
//
// Material 3 dark-first design with excellent contrast for readability.
// Optimized for fast rendering and accessibility.

import 'package:flutter/material.dart';

// UOT color palette — deep blues, cyans, and purples.
class UotColors {
  UotColors._();

  // === Primary Palette ===
  static const Color primaryDark = Color(0xFF0D1B2A);
  static const Color primary = Color(0xFF1B2838);
  static const Color primaryLight = Color(0xFF2C3E50);

  // === Accent Colors ===
  static const Color accent = Color(0xFF00D4AA);
  static const Color accentLight = Color(0xFF4AEDC4);
  static const Color accentDim = Color(0xFF008B71);

  // === Secondary Accent ===
  static const Color secondary = Color(0xFF7C4DFF);
  static const Color secondaryLight = Color(0xFFB388FF);

  // === Surface Colors ===
  static const Color surfaceDark = Color(0xFF0A1628);
  static const Color surface = Color(0xFF121E2E);
  static const Color surfaceLight = Color(0xFF1A2B3F);
  static const Color surfaceElevated = Color(0xFF223347);

  // === Text Colors (high contrast) ===
  static const Color textPrimary = Color(0xFFF0F4F8);
  static const Color textSecondary = Color(0xFFB0BEC5);
  static const Color textTertiary = Color(0xFF78909C);
  static const Color textOnAccent = Color(0xFF0D1B2A);

  // === Status Colors ===
  static const Color success = Color(0xFF4CAF50);
  static const Color warning = Color(0xFFFF9800);
  static const Color error = Color(0xFFEF5350);
  static const Color info = Color(0xFF42A5F5);

  // === Transfer Status ===
  static const Color transferActive = Color(0xFF00D4AA);
  static const Color transferPaused = Color(0xFFFF9800);
  static const Color transferComplete = Color(0xFF4CAF50);
  static const Color transferFailed = Color(0xFFEF5350);

  // === Light Theme Colors ===
  static const Color lightBackground = Color(0xFFF5F7FA);
  static const Color lightSurface = Color(0xFFFFFFFF);
  static const Color lightSurfaceElevated = Color(0xFFF0F2F5);
  static const Color lightTextPrimary = Color(0xFF1A1A2E);
  static const Color lightTextSecondary = Color(0xFF4A5568);
}

// The UOT theme configuration.
class UotTheme {
  UotTheme._();

  /// Dark theme (default).
  static ThemeData get dark {
    return ThemeData(
      useMaterial3: true,
      brightness: Brightness.dark,
      fontFamily: 'Inter',
      colorScheme: const ColorScheme.dark(
        primary: UotColors.accent,
        onPrimary: UotColors.textOnAccent,
        secondary: UotColors.secondary,
        onSecondary: Colors.white,
        surface: UotColors.surface,
        onSurface: UotColors.textPrimary,
        error: UotColors.error,
        onError: Colors.white,
      ),
      scaffoldBackgroundColor: UotColors.surfaceDark,
      appBarTheme: const AppBarTheme(
        backgroundColor: Colors.transparent,
        elevation: 0,
        scrolledUnderElevation: 0,
        centerTitle: false,
        titleTextStyle: TextStyle(
          color: UotColors.textPrimary,
          fontSize: 20,
          fontWeight: FontWeight.w600,
          letterSpacing: -0.3,
        ),
        iconTheme: IconThemeData(color: UotColors.textPrimary),
      ),
      cardTheme: CardThemeData(
        color: UotColors.surface,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: BorderSide(
            color: UotColors.surfaceElevated.withOpacity(0.5),
          ),
        ),
      ),
      navigationBarTheme: NavigationBarThemeData(
        backgroundColor: UotColors.surface,
        indicatorColor: UotColors.accent.withOpacity(0.15),
        labelTextStyle: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.selected)) {
            return const TextStyle(
              color: UotColors.accent,
              fontSize: 12,
              fontWeight: FontWeight.w600,
            );
          }
          return const TextStyle(
            color: UotColors.textTertiary,
            fontSize: 12,
            fontWeight: FontWeight.w500,
          );
        }),
        iconTheme: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.selected)) {
            return const IconThemeData(color: UotColors.accent, size: 24);
          }
          return const IconThemeData(color: UotColors.textTertiary, size: 24);
        }),
      ),
      navigationRailTheme: NavigationRailThemeData(
        backgroundColor: UotColors.surface,
        indicatorColor: UotColors.accent.withOpacity(0.15),
        selectedIconTheme: const IconThemeData(color: UotColors.accent),
        unselectedIconTheme: const IconThemeData(color: UotColors.textTertiary),
        selectedLabelTextStyle: const TextStyle(
          color: UotColors.accent,
          fontSize: 12,
          fontWeight: FontWeight.w600,
        ),
        unselectedLabelTextStyle: const TextStyle(
          color: UotColors.textTertiary,
          fontSize: 12,
        ),
      ),
      textTheme: const TextTheme(
        displayLarge: TextStyle(
          color: UotColors.textPrimary,
          fontWeight: FontWeight.w700,
          letterSpacing: -0.5,
        ),
        headlineLarge: TextStyle(
          color: UotColors.textPrimary,
          fontWeight: FontWeight.w600,
          letterSpacing: -0.3,
        ),
        headlineMedium: TextStyle(
          color: UotColors.textPrimary,
          fontWeight: FontWeight.w600,
        ),
        titleLarge: TextStyle(
          color: UotColors.textPrimary,
          fontWeight: FontWeight.w600,
        ),
        titleMedium: TextStyle(
          color: UotColors.textPrimary,
          fontWeight: FontWeight.w500,
        ),
        bodyLarge: TextStyle(
          color: UotColors.textPrimary,
          fontSize: 16,
          height: 1.5,
        ),
        bodyMedium: TextStyle(
          color: UotColors.textSecondary,
          fontSize: 14,
          height: 1.4,
        ),
        bodySmall: TextStyle(color: UotColors.textTertiary, fontSize: 12),
        labelLarge: TextStyle(
          color: UotColors.textPrimary,
          fontWeight: FontWeight.w600,
          fontSize: 14,
        ),
      ),
      iconTheme: const IconThemeData(color: UotColors.textSecondary, size: 24),
      dividerTheme: DividerThemeData(
        color: UotColors.surfaceElevated.withOpacity(0.5),
        thickness: 1,
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: UotColors.accent,
          foregroundColor: UotColors.textOnAccent,
          elevation: 0,
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
          textStyle: const TextStyle(fontWeight: FontWeight.w600, fontSize: 14),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: UotColors.accent,
          side: const BorderSide(color: UotColors.accent, width: 1.5),
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
        ),
      ),
      chipTheme: ChipThemeData(
        backgroundColor: UotColors.surfaceLight,
        selectedColor: UotColors.accent.withOpacity(0.15),
        labelStyle: const TextStyle(color: UotColors.textSecondary),
        side: BorderSide.none,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      ),
      progressIndicatorTheme: const ProgressIndicatorThemeData(
        color: UotColors.accent,
        linearTrackColor: UotColors.surfaceElevated,
      ),
      snackBarTheme: SnackBarThemeData(
        backgroundColor: UotColors.surfaceElevated,
        contentTextStyle: const TextStyle(color: UotColors.textPrimary),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        behavior: SnackBarBehavior.floating,
      ),
    );
  }

  /// Light theme.
  static ThemeData get light {
    return ThemeData(
      useMaterial3: true,
      brightness: Brightness.light,
      fontFamily: 'Inter',
      colorScheme: const ColorScheme.light(
        primary: UotColors.accentDim,
        onPrimary: Colors.white,
        secondary: UotColors.secondary,
        onSecondary: Colors.white,
        surface: UotColors.lightSurface,
        onSurface: UotColors.lightTextPrimary,
        error: UotColors.error,
      ),
      scaffoldBackgroundColor: UotColors.lightBackground,
      appBarTheme: const AppBarTheme(
        backgroundColor: Colors.transparent,
        elevation: 0,
        scrolledUnderElevation: 0,
        centerTitle: false,
        titleTextStyle: TextStyle(
          color: UotColors.lightTextPrimary,
          fontSize: 20,
          fontWeight: FontWeight.w600,
          letterSpacing: -0.3,
        ),
        iconTheme: IconThemeData(color: UotColors.lightTextPrimary),
      ),
      cardTheme: CardThemeData(
        color: UotColors.lightSurface,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: BorderSide(color: Colors.grey.shade200),
        ),
      ),
      textTheme: const TextTheme(
        bodyLarge: TextStyle(
          color: UotColors.lightTextPrimary,
          fontSize: 16,
          height: 1.5,
        ),
        bodyMedium: TextStyle(
          color: UotColors.lightTextSecondary,
          fontSize: 14,
          height: 1.4,
        ),
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: UotColors.accentDim,
          foregroundColor: Colors.white,
          elevation: 0,
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
        ),
      ),
      progressIndicatorTheme: const ProgressIndicatorThemeData(
        color: UotColors.accentDim,
      ),
    );
  }
}
