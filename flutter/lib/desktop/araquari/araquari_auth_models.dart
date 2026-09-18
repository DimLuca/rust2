enum AraquariRole { admin, technician }

class AraquariUser {
  const AraquariUser({
    required this.id,
    required this.username,
    required this.displayName,
    required this.role,
  });

  factory AraquariUser.fromJson(Map<String, dynamic> json) {
    return AraquariUser(
      id: json['id'] as String,
      username: json['username'] as String,
      displayName: json['display_name'] as String,
      role: json['role'] == 'ADMIN'
          ? AraquariRole.admin
          : AraquariRole.technician,
    );
  }

  final String id;
  final String username;
  final String displayName;
  final AraquariRole role;
}

class AraquariSession {
  const AraquariSession({
    required this.accessToken,
    required this.expiresAt,
    required this.user,
  });

  factory AraquariSession.fromJson(Map<String, dynamic> json) {
    return AraquariSession(
      accessToken: json['access_token'] as String,
      expiresAt: DateTime.parse(json['expires_at'] as String).toUtc(),
      user: AraquariUser.fromJson(json['user'] as Map<String, dynamic>),
    );
  }

  final String accessToken;
  final DateTime expiresAt;
  final AraquariUser user;

  bool get isExpired => !DateTime.now()
      .toUtc()
      .isBefore(expiresAt.subtract(const Duration(seconds: 5)));
}

class AraquariDeviceIdentity {
  const AraquariDeviceIdentity({
    this.deviceId,
    this.hostname,
    this.osUsername,
    this.localIp,
    this.rustDeskId,
  });

  final String? deviceId;
  final String? hostname;
  final String? osUsername;
  final String? localIp;
  final String? rustDeskId;

  Map<String, dynamic> toJson() => {
        if (deviceId != null) 'device_id': deviceId,
        if (hostname != null) 'hostname': hostname,
        if (osUsername != null) 'os_username': osUsername,
        if (localIp != null) 'local_ip': localIp,
        if (rustDeskId != null) 'rustdesk_id': rustDeskId,
      };
}

enum AraquariAuthErrorKind {
  invalidCredentials,
  rateLimited,
  serviceUnavailable,
  invalidResponse,
}

class AraquariAuthException implements Exception {
  const AraquariAuthException(this.kind, [this.detail]);

  final AraquariAuthErrorKind kind;
  final String? detail;
}

