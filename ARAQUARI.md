# Personalização institucional de Araquari

Esta distribuição mantém a base técnica do RustDesk e aplica a configuração da
Prefeitura na inicialização do cliente.

## Configuração incorporada

- servidor de ID: `jiraiya.araquari.sc.gov.br`;
- servidor relay: `jiraiya.araquari.sc.gov.br`;
- chave pública do servidor: incorporada em `src/araquari.rs`;
- cor principal: azul institucional `#27457E`;
- logos para temas claro e escuro e ícones dos pacotes: derivados da identidade
  visual fornecida pela Prefeitura, sem os elementos vermelhos.

O mesmo domínio atende os usuários internos e externos por DNS dividido:

- DNS interno: `jiraiya.araquari.sc.gov.br` deve apontar para `192.168.0.55`;
- DNS público: o mesmo nome deve apontar para o IP público que publica o
  RustDesk Server.

## Geração dos aplicativos

O workflow `Flutter Tag Build` já existente gera os pacotes do projeto. Para
produzir uma versão, crie uma tag no formato `vX.Y.Z` ou execute o workflow
manualmente em **Actions**. Os instaladores ficam nos artefatos e na release
correspondente.

Windows, Linux e macOS exigem runners diferentes; por isso a validação completa
dos instaladores ocorre no GitHub Actions, não em uma única máquina local.

## Acesso restrito da TI

O aplicativo sempre inicia no modo Usuário. O modo TI libera as ferramentas de
conexão de saída e as configurações somente após autenticação na AraquariDesk
API. Não existe usuário, senha ou hash administrativo no Flutter ou no GitHub.

Configure a variável de repositório `ARAQUARIDESK_API_URL` com o endpoint HTTPS.
As instruções de PostgreSQL, criação de administradores e técnicos, alteração de
senha, auditoria e reverse proxy estão em `araquaridesk-server/README.md`.

No modo Usuário, a janela é compacta e mostra somente o painel de recebimento de
suporte. Após o login da TI, a mesma janela expande e apresenta o painel de
controle remoto. Sair da TI revoga a sessão e retorna imediatamente ao layout
compacto.

## Assinatura do Windows

O pipeline aceita um certificado Authenticode por meio dos segredos
`WINDOWS_CODESIGN_PFX_BASE64` e `WINDOWS_CODESIGN_PFX_PASSWORD`. A URL do
servidor de timestamp pode ser definida na variável
`WINDOWS_CODESIGN_TIMESTAMP_URL`. A chave privada nunca deve ser adicionada ao
repositório.

Sem certificado, o workflow informa claramente que os pacotes estão sem
assinatura. Nenhuma proteção do Windows é desativada. A remoção consistente dos
alertas do SmartScreen depende de certificado confiável, timestamp e reputação
do editor.

## Download bloqueado no Windows Defender (sem certificado)

Em PC limpo e fora da rede, o Defender/SmartScreen pode **apagar ou recusar o
download** do instalador branco `.msi`/`.exe`. Isso não é, por si só, sinal de
malware no código: desktop remoto sem assinatura e sem reputação entra na mesma
classe de heurística que RATs.

Por que “antes funcionava e depois parou”: builds oficiais do RustDesk (ou
versões anteriores com outro hash) acumulavam reputação na nuvem da Microsoft.
Cada build novo do fork gera um hash desconhecido; depois do rebrand e dos
ajustes, o Windows passou a tratar o pacote como arquivo novo de risco.

### Canais de entrega (sem certificado pago)

Muitos PCs **não passam pela TI**. O canal principal é o que o usuário final
recebe e abre **sem administrador**.

| Artefato | Para quem | Admin? | Uso |
|----------|-----------|--------|-----|
| **`AraquariDesk-portavel-<versão>-<arch>.zip`** | Usuário final | **Não** | **Canal principal (WhatsApp)** |
| `AraquariDesk-Setup-*.exe` | Usuário final | **Não** (extrai e roda em modo Usuário) | Alternativa em arquivo único |
| `AraquariDesk-<versão>-<arch>.msi` | TI / GPO | Sim | Instalação permanente + serviço |
| `AraquariDesk-<versão>-<arch>.7z` | TI | Não para extrair | Fallback se o Defender apagar o download do zip/msi |
| `SHA256SUMS-<arch>.txt` | TI | — | Conferência de hash |

#### Fluxo WhatsApp / campo (sem admin)

1. Enviar o **`AraquariDesk-portavel-*.zip`** (ou o `Setup-*.exe`) pelo WhatsApp.
2. No PC do usuário: **Extrair tudo** no ZIP do Windows (sem senha, sem admin).
3. Abrir `AraquariDesk.exe` / `rustdesk.exe`.
4. SmartScreen: **Mais informações → Executar assim mesmo**.
5. Modo Usuário: passar o **ID** da tela para o técnico e **aceitar** a conexão.

O ZIP inclui `COMO-USAR.txt` com esse passo a passo.

Isso cobre **suporte assistido** (app aberto, usuário aceita). **Não** cobre
acesso desatendido permanente: serviço/MSI continuam sendo da TI, com admin.

#### Se o Defender apagar o download

1. Preferir o `.zip` em vez do `.exe` (ZIP costuma passar mais).
2. Baixar via PowerShell se o navegador bloquear:

   ```powershell
   Invoke-WebRequest -Uri '<url-do-zip>' -OutFile 'AraquariDesk-portavel.zip'
   ```

3. Fallback TI: `.7z` com senha `AraquariDesk-TI` (segredo
   `ARAQUARIDESK_DIST_ZIP_PASSWORD` no GitHub).
4. **Não** desativar o Defender inteiro. Em teste, desligar só tempo real / PUA
   / nuvem, extrair e religar.
5. A cada release, submeter o `.zip`/`.msi` ao portal gratuito da Microsoft:
   <https://www.microsoft.com/wdsi/filesubmission>.
6. Hospedar o `.zip` também em `jiraiya.araquari.sc.gov.br` e mandar esse link
   quando o WhatsApp/GitHub bloquear.

#### Preferências

- **Usuário final / WhatsApp:** portátil `.zip` (ou Setup exe) — sem admin.
- **Estações da TI / GPO:** MSI (precisa admin) — não é o canal de campo.
- Não usar desativação permanente do antivírus como procedimento padrão.
