// UOT Application Root
//
// Configures MaterialApp with theme, routing, and responsive layout.

import 'package:flutter/material.dart';
import 'src/core/theme/app_theme.dart';
import 'src/core/router/app_router.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  try {
    await RustLib.init();
  } catch (e, stackTrace) {
    debugPrint('UOT RustLib initialization error: $e\n$stackTrace');
  }

  runApp(const UotApp());
}

// Root application widget.
class UotApp extends StatefulWidget {
  const UotApp({super.key});

  @override
  State<UotApp> createState() => _UotAppState();
}

class _UotAppState extends State<UotApp> {
  ThemeMode _themeMode = ThemeMode.dark;

  void _toggleTheme() {
    setState(() {
      _themeMode =
          _themeMode == ThemeMode.dark ? ThemeMode.light : ThemeMode.dark;
    });
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'UOT',
      debugShowCheckedModeBanner: false,
      theme: UotTheme.light,
      darkTheme: UotTheme.dark,
      themeMode: _themeMode,
      home: AppRouter(onToggleTheme: _toggleTheme),
    );
  }
}
