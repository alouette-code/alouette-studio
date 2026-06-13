import 'package:flutter/material.dart';
import '../theme/app_theme.dart';

class MiniPostman extends StatelessWidget {
  const MiniPostman({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Container(
        color: Theme.of(context).brightness == Brightness.dark
            ? const Color(0xFF111118)
            : const Color(0xFFF5F5F8),
        child: const Center(
          child: Text('MiniPostman - Coming soon'),
        ),
      ),
    );
  }
}
