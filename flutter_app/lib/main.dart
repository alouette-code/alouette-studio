import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:window_manager/window_manager.dart';
import 'src/providers/app_provider.dart';
import 'src/services/rust_bridge_service.dart';
import 'src/theme/app_theme.dart';
import 'src/app.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Khởi tạo window_manager để ẩn thanh tiêu đề OS
  await windowManager.ensureInitialized();

  const windowOptions = WindowOptions(
    size: Size(1280, 800),
    minimumSize: Size(800, 600),
    center: true,
    title: 'Alouette Studio',
    titleBarStyle: TitleBarStyle.hidden, // Ẩn hoàn toàn thanh tiêu đề OS
    windowButtonVisibility: false,
  );

  windowManager.waitUntilReadyToShow(windowOptions, () async {
    await windowManager.show();
    await windowManager.focus();
  });

  final bridge = PlaceholderRustBridge();

  runApp(
    ChangeNotifierProvider(
      create: (_) => AppProvider(bridge)..loadProjects(),
      child: const AlouetteStudioApp(),
    ),
  );
}

class AlouetteStudioApp extends StatelessWidget {
  const AlouetteStudioApp({super.key});

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) {
        return MaterialApp(
          title: 'Alouette Studio',
          debugShowCheckedModeBanner: false,
          theme: AppTheme.darkTheme(),
          darkTheme: AppTheme.darkTheme(),
          themeMode: provider.isDark ? ThemeMode.dark : ThemeMode.light,
          home: const AlouetteStudioHome(),
        );
      },
    );
  }
}
