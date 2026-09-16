import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/consts.dart';
import 'package:flutter_hbb/desktop/araquari/araquari_access_controller.dart';
import 'package:flutter_hbb/desktop/araquari/araquari_ti_login_dialog.dart';
import 'package:flutter_hbb/desktop/pages/connection_page.dart';
import 'package:flutter_hbb/desktop/pages/desktop_setting_page.dart';
import 'package:flutter_hbb/models/server_model.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:provider/provider.dart';

class AraquariHomeLayout extends StatelessWidget {
  const AraquariHomeLayout({
    super.key,
    required this.systemError,
    required this.serviceStopped,
  });

  final String systemError;
  final bool serviceStopped;

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: AraquariAccessController.instance,
      builder: (context, _) {
        final tiMode = AraquariAccessController.instance.isTiMode;
        return ChangeNotifierProvider.value(
          value: gFFI.serverModel,
          child: Consumer<ServerModel>(
            builder: (context, model, _) => _buildBackground(
              context,
              model,
              tiMode,
            ),
          ),
        );
      },
    );
  }

  Widget _buildBackground(
    BuildContext context,
    ServerModel model,
    bool tiMode,
  ) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    return Stack(
      fit: StackFit.expand,
      children: [
        Image.asset(
          dark ? 'assets/preset_dark.jpg' : 'assets/preset_light.jpg',
          fit: BoxFit.cover,
          filterQuality: FilterQuality.medium,
        ),
        DecoratedBox(
          decoration: BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topCenter,
              end: Alignment.bottomCenter,
              colors: dark
                  ? const [Color(0xC90B1730), Color(0xEE071226)]
                  : const [Color(0x8AFFFFFF), Color(0xE8EDF4FC)],
            ),
          ),
        ),
        SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(22),
            child: Column(
              children: [
                _Header(tiMode: tiMode),
                const SizedBox(height: 18),
                Expanded(
                  child: LayoutBuilder(
                    builder: (context, constraints) {
                      final compact = constraints.maxWidth < 880;
                      final supportCard = _SupportCard(
                        model: model,
                        systemError: systemError,
                        serviceStopped: serviceStopped,
                        tiMode: tiMode,
                      );
                      if (!tiMode) {
                        return Center(
                          child: ConstrainedBox(
                            constraints: const BoxConstraints(maxWidth: 680),
                            child: supportCard,
                          ),
                        );
                      }
                      if (compact) {
                        return ListView(
                          children: [
                            supportCard,
                            const SizedBox(height: 16),
                            SizedBox(height: 520, child: _ConnectionCard()),
                          ],
                        );
                      }
                      return Row(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          SizedBox(width: 410, child: supportCard),
                          const SizedBox(width: 18),
                          const Expanded(child: _ConnectionCard()),
                        ],
                      );
                    },
                  ),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.tiMode});

  final bool tiMode;

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    return Row(
      children: [
        SvgPicture.asset(
          'assets/araquari_desk.svg',
          height: 42,
          colorFilter: dark
              ? const ColorFilter.mode(Colors.white, BlendMode.srcIn)
              : null,
        ),
        const Spacer(),
        if (tiMode)
          Container(
            margin: const EdgeInsets.only(right: 10),
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            decoration: BoxDecoration(
              color: MyTheme.accent.withOpacity(0.13),
              borderRadius: BorderRadius.circular(999),
              border: Border.all(color: MyTheme.accent.withOpacity(0.28)),
            ),
            child: const Row(
              children: [
                Icon(Icons.admin_panel_settings_outlined, size: 17),
                SizedBox(width: 7),
                Text('Modo TI'),
              ],
            ),
          ),
        IconButton(
          tooltip: dark ? 'Usar tema claro' : 'Usar tema escuro',
          onPressed: () => MyTheme.changeDarkMode(
            dark ? ThemeMode.light : ThemeMode.dark,
          ),
          icon: Icon(dark ? Icons.light_mode_outlined : Icons.dark_mode_outlined),
        ),
        const SizedBox(width: 4),
        if (tiMode)
          TextButton.icon(
            onPressed: AraquariAccessController.instance.logout,
            icon: const Icon(Icons.logout, size: 17),
            label: const Text('Sair da TI'),
          )
        else
          TextButton.icon(
            onPressed: () => showAraquariTiLoginDialog(context),
            icon: const Icon(Icons.lock_outline, size: 16),
            label: const Text('Acesso da TI'),
          ),
      ],
    );
  }
}

class _SupportCard extends StatelessWidget {
  const _SupportCard({
    required this.model,
    required this.systemError,
    required this.serviceStopped,
    required this.tiMode,
  });

  final ServerModel model;
  final String systemError;
  final bool serviceStopped;
  final bool tiMode;

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final showOneTime = model.approveMode != 'click' &&
        model.verificationMethod != kUsePermanentPassword;
    final ready = systemError.isEmpty && !serviceStopped;
    final surface = dark
        ? const Color(0xE8142139)
        : const Color(0xF7FFFFFF);
    return Container(
      padding: const EdgeInsets.all(28),
      decoration: BoxDecoration(
        color: surface,
        borderRadius: BorderRadius.circular(20),
        border: Border.all(
          color: dark ? Colors.white12 : const Color(0xFFD7E2F0),
        ),
        boxShadow: const [
          BoxShadow(
            color: Color(0x24051224),
            blurRadius: 28,
            offset: Offset(0, 12),
          ),
        ],
      ),
      child: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Suporte remoto',
              style: Theme.of(context).textTheme.titleLarge?.copyWith(
                    fontSize: 25,
                    fontWeight: FontWeight.w700,
                  ),
            ),
            const SizedBox(height: 8),
            Text(
              'Informe à equipe de TI os dados deste computador somente durante um atendimento autorizado.',
              style: Theme.of(context)
                  .textTheme
                  .bodyMedium
                  ?.copyWith(height: 1.5),
            ),
            const SizedBox(height: 24),
            _StatusBanner(
              ready: ready,
              message: serviceStopped
                  ? 'Serviço de suporte interrompido'
                  : systemError.isNotEmpty
                      ? systemError
                      : 'Pronto para receber suporte',
            ),
            const SizedBox(height: 22),
            _CredentialField(
              label: 'ID deste computador',
              value: model.serverId.text.isEmpty
                  ? 'Aguardando conexão...'
                  : model.serverId.text,
              enabled: model.serverId.text.isNotEmpty,
              icon: Icons.desktop_windows_outlined,
            ),
            const SizedBox(height: 14),
            _CredentialField(
              label: 'Senha temporária',
              value: showOneTime && model.serverPasswd.text.isNotEmpty
                  ? model.serverPasswd.text
                  : 'Aprovação manual ativada',
              enabled: showOneTime && model.serverPasswd.text.isNotEmpty,
              icon: Icons.password_outlined,
              onRefresh: showOneTime
                  ? () => bind.mainUpdateTemporaryPassword()
                  : null,
            ),
            const SizedBox(height: 22),
            Container(
              padding: const EdgeInsets.all(14),
              decoration: BoxDecoration(
                color: MyTheme.accent.withOpacity(dark ? 0.16 : 0.07),
                borderRadius: BorderRadius.circular(10),
              ),
              child: const Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(Icons.shield_outlined, size: 20),
                  SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      'Você sempre verá uma solicitação antes de uma conexão que exigir aprovação.',
                    ),
                  ),
                ],
              ),
            ),
            if (tiMode && !bind.isDisableSettings()) ...[
              const SizedBox(height: 18),
              OutlinedButton.icon(
                onPressed: () => DesktopSettingPage.switch2page(
                  SettingsTabKey.general,
                ),
                icon: const Icon(Icons.settings_outlined, size: 18),
                label: const Text('Configurações técnicas'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _StatusBanner extends StatelessWidget {
  const _StatusBanner({required this.ready, required this.message});

  final bool ready;
  final String message;

  @override
  Widget build(BuildContext context) {
    final color = ready ? const Color(0xFF16815D) : const Color(0xFFB26A12);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 13, vertical: 11),
      decoration: BoxDecoration(
        color: color.withOpacity(0.12),
        borderRadius: BorderRadius.circular(9),
        border: Border.all(color: color.withOpacity(0.35)),
      ),
      child: Row(
        children: [
          Icon(ready ? Icons.check_circle_outline : Icons.info_outline,
              color: color, size: 19),
          const SizedBox(width: 9),
          Expanded(child: Text(message, maxLines: 2)),
        ],
      ),
    );
  }
}

class _CredentialField extends StatelessWidget {
  const _CredentialField({
    required this.label,
    required this.value,
    required this.enabled,
    required this.icon,
    this.onRefresh,
  });

  final String label;
  final String value;
  final bool enabled;
  final IconData icon;
  final VoidCallback? onRefresh;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.fromLTRB(16, 13, 10, 13),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.background.withOpacity(0.72),
        borderRadius: BorderRadius.circular(11),
        border: Border.all(color: Theme.of(context).dividerColor),
      ),
      child: Row(
        children: [
          Icon(icon, color: MyTheme.accent),
          const SizedBox(width: 13),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(label, style: Theme.of(context).textTheme.bodySmall),
                const SizedBox(height: 4),
                SelectableText(
                  value,
                  maxLines: 1,
                  style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w600,
                    letterSpacing: 0.4,
                  ),
                ),
              ],
            ),
          ),
          if (onRefresh != null)
            IconButton(
              tooltip: 'Gerar nova senha',
              onPressed: onRefresh,
              icon: const Icon(Icons.refresh),
            ),
          IconButton(
            tooltip: 'Copiar',
            onPressed: enabled
                ? () {
                    Clipboard.setData(ClipboardData(text: value));
                    showToast('Copiado');
                  }
                : null,
            icon: const Icon(Icons.copy_outlined),
          ),
        ],
      ),
    );
  }
}

class _ConnectionCard extends StatelessWidget {
  const _ConnectionCard();

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    return Container(
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: dark ? const Color(0xF2131D31) : const Color(0xF8FFFFFF),
        borderRadius: BorderRadius.circular(20),
        border: Border.all(
          color: dark ? Colors.white12 : const Color(0xFFD7E2F0),
        ),
      ),
      child: const ConnectionPage(),
    );
  }
}
