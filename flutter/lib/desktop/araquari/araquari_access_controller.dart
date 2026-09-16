import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:flutter/foundation.dart';

typedef AraquariCredentialVerifier = bool Function(
  String username,
  String password,
);

/// Keeps the restricted TI session in memory only.
///
/// The verifier is deliberately isolated so it can later be replaced by an
/// API, LDAP or another institutional identity provider.
class AraquariAccessController extends ChangeNotifier {
  AraquariAccessController({AraquariCredentialVerifier? verifier})
      : _verifier = verifier ?? _LocalCredentialVerifier.verify;

  static final instance = AraquariAccessController();

  final AraquariCredentialVerifier _verifier;
  bool _isTiMode = false;

  bool get isTiMode => _isTiMode;

  bool authenticate(String username, String password) {
    final authenticated = _verifier(username.trim(), password);
    if (authenticated != _isTiMode) {
      _isTiMode = authenticated;
      notifyListeners();
    }
    return authenticated;
  }

  void logout() {
    if (!_isTiMode) return;
    _isTiMode = false;
    notifyListeners();
  }
}

class _LocalCredentialVerifier {
  static const _usernameDigest = String.fromEnvironment(
    'ARAQUARI_TI_USERNAME_SHA256',
  );
  static const _passwordDigest = String.fromEnvironment(
    'ARAQUARI_TI_PASSWORD_SHA256',
  );

  static bool verify(String username, String password) {
    if (_usernameDigest.isEmpty || _passwordDigest.isEmpty) return false;
    final usernameDigest = sha256.convert(utf8.encode(username)).toString();
    final passwordDigest = sha256.convert(utf8.encode(password)).toString();
    final usernameMatches =
        _constantTimeEquals(usernameDigest, _usernameDigest);
    final passwordMatches =
        _constantTimeEquals(passwordDigest, _passwordDigest);
    return usernameMatches && passwordMatches;
  }

  static bool _constantTimeEquals(String left, String right) {
    if (left.length != right.length) return false;
    var difference = 0;
    for (var index = 0; index < left.length; index++) {
      difference |= left.codeUnitAt(index) ^ right.codeUnitAt(index);
    }
    return difference == 0;
  }
}
