import 'package:flutter/material.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:window_manager/window_manager.dart';

class AraquariWindowController {
  const AraquariWindowController._();

  static const userModeSize = Size(520, 720);
  static const tiModeSize = Size(1000, 720);
  static const minimumSize = Size(460, 640);
  static bool _resizing = false;

  static Future<void> applyMode(bool tiMode) async {
    if (!isDesktop || _resizing) return;
    _resizing = true;
    try {
      if (await windowManager.isMaximized() ||
          await windowManager.isFullScreen()) {
        return;
      }
      await windowManager.setMinimumSize(minimumSize);
      await windowManager.setSize(tiMode ? tiModeSize : userModeSize);
      // window_manager works in logical pixels and centers inside the active
      // display work area, respecting DPI scaling and task bars.
      await windowManager.center();
    } catch (error) {
      debugPrint('AraquariDesk window resize error: $error');
    } finally {
      _resizing = false;
    }
  }
}

