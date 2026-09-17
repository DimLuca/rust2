class AraquariApiConfig {
  const AraquariApiConfig._();

  static const baseUrl = String.fromEnvironment(
    'ARAQUARIDESK_API_URL',
    defaultValue: 'https://jiraiya.araquari.sc.gov.br/araquaridesk-api',
  );

  static Uri endpoint(String path) {
    final normalizedBase = baseUrl.endsWith('/')
        ? baseUrl.substring(0, baseUrl.length - 1)
        : baseUrl;
    return Uri.parse('$normalizedBase$path');
  }
}

