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

### Canal de entrega sem pagar certificado

Cada release do Windows publica, além do `.msi` e do `.exe` portátil:

| Artefato | Uso |
|----------|-----|
| `AraquariDesk-<versão>-<arch>.7z` | **Canal preferencial** em PC limpo: pacote com senha |
| `AraquariDesk-<versão>-<arch>.msi` | Instalação institucional (rede interna / GPO) |
| `SHA256SUMS-<arch>.txt` | Conferência de integridade pela TI |
| `AraquariDesk-Setup-*.exe` | Evitar em PC limpo (self-extractor gera mais bloqueio) |

Senha do `.7z`: segredo `ARAQUARIDESK_DIST_ZIP_PASSWORD` no GitHub. Se o
segredo não existir, o pipeline usa `AraquariDesk-TI`. A senha é só para
descompactar após o download; não substitui assinatura Authenticode.

Arquivo protegido por senha (cabeçalho 7z criptografado com `-mhe=on`) costuma
passar no download mesmo quando o Defender apagaria o `.msi` nu.

### Procedimento da TI quando o download for bloqueado

1. Baixar o `.7z` (não o `.exe` Setup) da release e, se possível, o
   `SHA256SUMS-<arch>.txt`.
2. Se o navegador bloquear, copiar o link e baixar com PowerShell:

   ```powershell
   Invoke-WebRequest -Uri '<url-do-7z>' -OutFile 'AraquariDesk.7z'
   ```

3. Descompactar com 7-Zip usando a senha da TI.
4. Conferir o hash:

   ```powershell
   Get-FileHash -Algorithm SHA256 .\AraquariDesk-*.msi
   ```

5. Instalar o `.msi`. A tela do SmartScreen na execução (“Mais informações →
   Executar assim mesmo”) é esperada sem certificado; o bloqueio **durante o
   download** é o que o `.7z` resolve.
6. Se ainda assim o arquivo sumir após o download, **não desativar o Defender
   inteiro**. Em máquina de teste, desligar só:
   - Proteção em tempo real;
   - Bloqueio de aplicativos potencialmente indesejados (PUA);
   - Proteção baseada em nuvem.
   Depois reinstalar e religar as opções.
7. A cada release, submeter o `.msi`/`.7z` ao portal gratuito da Microsoft:
   <https://www.microsoft.com/wdsi/filesubmission>. Isso reduz bloqueios nos
   próximos PCs limpos **sem comprar certificado**.
8. Alternativa de campo: gravar o `.7z` + `SHA256SUMS` em USB e instalar offline,
   evitando o caminho do navegador.

### Preferências de empacotagem

- Para estações da Prefeitura, distribuir o **MSI** (direto ou de dentro do
  `.7z`), de preferência por share interno/GPO quando o PC estiver na rede.
- Publicar também o `.7z` no portal institucional em
  `jiraiya.araquari.sc.gov.br`, além da release do GitHub — domínio próprio
  tem reputação diferente de anexos de release pública.
- Não usar desativação permanente do antivírus como procedimento padrão.
