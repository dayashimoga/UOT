import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:uot_app/src/core/theme/app_theme.dart';

void main() {
  group('UotTheme', () {
    test('dark theme uses Material 3', () {
      final theme = UotTheme.dark;
      expect(theme.useMaterial3, isTrue);
      expect(theme.brightness, Brightness.dark);
    });

    test('light theme uses Material 3', () {
      final theme = UotTheme.light;
      expect(theme.useMaterial3, isTrue);
      expect(theme.brightness, Brightness.light);
    });

    test('dark theme has high contrast text', () {
      final theme = UotTheme.dark;
      // Primary text should be very light for dark theme (high contrast)
      final textColor = theme.textTheme.bodyLarge?.color;
      expect(textColor, isNotNull);
      // Luminance > 0.8 means very light (white-ish)
      expect(textColor!.computeLuminance(), greaterThan(0.8));
    });

    test('dark theme primary color is accent green/cyan', () {
      final theme = UotTheme.dark;
      expect(theme.colorScheme.primary, equals(UotColors.accent));
    });

    test('dark scaffold background is very dark', () {
      final theme = UotTheme.dark;
      final bg = theme.scaffoldBackgroundColor;
      expect(bg.computeLuminance(), lessThan(0.05));
    });
  });

  group('UotColors', () {
    test('text primary has high contrast on dark background', () {
      // Text primary luminance vs surface dark luminance
      final textLum = UotColors.textPrimary.computeLuminance();
      final bgLum = UotColors.surfaceDark.computeLuminance();
      // WCAG contrast ratio should be > 4.5 for AA
      final ratio = (textLum + 0.05) / (bgLum + 0.05);
      expect(ratio, greaterThan(4.5));
    });

    test('accent color is visible on dark background', () {
      final accentLum = UotColors.accent.computeLuminance();
      final bgLum = UotColors.surfaceDark.computeLuminance();
      final ratio = (accentLum + 0.05) / (bgLum + 0.05);
      expect(ratio, greaterThan(3.0));
    });

    test('status colors are distinct', () {
      expect(UotColors.success, isNot(equals(UotColors.error)));
      expect(UotColors.warning, isNot(equals(UotColors.error)));
      expect(UotColors.info, isNot(equals(UotColors.success)));
    });
  });
}
