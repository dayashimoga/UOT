// UOT Application Root
//
// Configures MaterialApp with theme, routing, and responsive layout.
// Handles Rust FFI engine initialization with proper error recovery.
// Engine init runs asynchronously with a timeout to prevent ANR on Android.

import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'src/core/theme/app_theme.dart';
import 'src/core/router/app_router.dart';
import 'src/features/diagnostics/rust_init_failed_screen.dart';
import 'src/rust/frb_generated.dart';
import 'src/rust/api/engine_api.dart' as engine;

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const UotApp());
}

/// Root application widget.
class UotApp extends StatefulWidget {
  const UotApp({super.key});

  @override
  State<UotApp> createState() => _UotAppState();
}

class _UotAppState extends State<UotApp> {
  ThemeMode _themeMode = ThemeMode.dark;

  /// Engine initialization state.
  _EngineInitState _engineState = _EngineInitState.loading;
  Object? _initError;
  StackTrace? _initStackTrace;

  @override
  void initState() {
    super.initState();
    _initializeEngine();
  }

  Future<void> _initializeEngine() async {
    setState(() {
      _engineState = _EngineInitState.loading;
      _initError = null;
      _initStackTrace = null;
    });

    try {
      // Run RustLib.init() with a 15-second timeout to prevent ANR.
      // On Android, native .so loading can hang if the library is missing
      // or the ABI is incompatible; the timeout ensures graceful recovery.
      await RustLib.init().timeout(
        const Duration(seconds: 15),
        onTimeout: () {
          throw TimeoutException(
            'Rust engine initialization timed out after 15 seconds. '
            'This may indicate a missing native library for this platform/ABI.',
          );
        },
      );
      if (mounted) {
        setState(() {
          _engineState = _EngineInitState.ready;
        });

        // Configure native storage save directory for incoming transfers
        try {
          Directory? saveDir;
          if (Platform.isAndroid) {
            try {
              saveDir = await getDownloadsDirectory();
            } catch (_) {}
            saveDir ??= await getApplicationDocumentsDirectory();
          } else if (Platform.isWindows ||
              Platform.isMacOS ||
              Platform.isLinux) {
            try {
              saveDir = await getDownloadsDirectory();
            } catch (_) {}
            saveDir ??= await getApplicationDocumentsDirectory();
          } else {
            saveDir = await getApplicationDocumentsDirectory();
          }
          if (!saveDir.existsSync()) {
            saveDir.createSync(recursive: true);
          }
          engine.engineSetSaveDirectory(saveDirectory: saveDir.path);
          debugPrint('Configured UOT save directory: ${saveDir.path}');
        } catch (e) {
          debugPrint('Could not set custom save directory: $e');
        }

        if (defaultTargetPlatform == TargetPlatform.windows) {
          // Automatically trigger Windows Firewall rules for inbound UOT connections
          Future.microtask(() {
            try {
              engine.engineFixWindowsFirewall();
            } catch (e) {
              debugPrint('Windows firewall rule auto-configuration status: $e');
            }
          });
        }
      }
    } catch (e, stackTrace) {
      debugPrint('UOT RustLib initialization error: $e\n$stackTrace');
      if (mounted) {
        setState(() {
          _engineState = _EngineInitState.failed;
          _initError = e;
          _initStackTrace = stackTrace;
        });
      }
    }
  }

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
      home: _buildHome(),
    );
  }

  Widget _buildHome() {
    switch (_engineState) {
      case _EngineInitState.loading:
        return const Scaffold(
          body: Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                CircularProgressIndicator(),
                SizedBox(height: 16),
                Text('Initializing transfer engine...'),
              ],
            ),
          ),
        );
      case _EngineInitState.failed:
        return RustInitFailedScreen(
          error: _initError ?? 'Unknown error',
          stackTrace: _initStackTrace ?? StackTrace.current,
          onRetry: _initializeEngine,
        );
      case _EngineInitState.ready:
        return AppRouter(onToggleTheme: _toggleTheme);
    }
  }
}

/// Engine initialization states.
enum _EngineInitState {
  loading,
  failed,
  ready,
}
