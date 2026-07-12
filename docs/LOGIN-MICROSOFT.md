# Login Microsoft — passo a passo do Azure

O app já tem o fluxo de login completo implementado (device code → Xbox Live →
XSTS → Minecraft). A única peça que falta é **sua**: um *Client ID* de um
aplicativo registrado no Azure, aprovado pela Mojang para usar a API do
Minecraft. É gratuito; a espera pela aprovação da Mojang é a parte demorada
(dias a semanas).

## Etapa 1 — Criar o registro no portal Azure (~10 minutos)

1. Acesse https://portal.azure.com e entre com qualquer conta Microsoft
   (a mesma do Minecraft serve). Não precisa de assinatura paga.
2. Na busca do topo, procure **"App registrations"** (Registros de aplicativo)
   e abra.
3. Clique em **"New registration"** (Novo registro):
   - **Name**: qualquer nome, ex. `Modrinth Replica do Bruno`
   - **Supported account types**: escolha
     **"Personal Microsoft accounts only"** (Somente contas pessoais da
     Microsoft) — importante, o login do Minecraft usa contas pessoais.
   - **Redirect URI**: deixe em branco.
   - Clique em **Register**.
4. Na página do app criado, copie o **"Application (client) ID"** — um UUID
   tipo `1a2b3c4d-...`. **Esse é o Client ID.**
5. No menu lateral, vá em **Authentication** (Autenticação):
   - Role até **"Advanced settings"** → **"Allow public client flows"**
     (Permitir fluxos de cliente público) e marque **Yes**.
   - Salve. (Sem isso o *device code flow* não funciona.)

## Etapa 2 — Pedir aprovação da Mojang (a parte que demora)

A API do Minecraft (`api.minecraftservices.com`) só aceita client IDs
aprovados. O formulário oficial está em:

https://help.minecraft.net/hc/en-us/articles/16254801392141-Minecraft-Launcher-and-Website-APIs

(Se o link mudar, procure por "Minecraft third party launcher API form".)

No formulário, informe:
- O **Client ID** copiado na Etapa 1
- Nome do projeto (ex.: "Réplica do Modrinth App — projeto de aprendizado")
- Descrição honesta: launcher pessoal, open source, para estudo
- Seu e-mail

A Mojang responde por e-mail. Enquanto a aprovação não chega, o login
retorna erro na etapa `login_with_xbox` — é esperado.

## Etapa 3 — Configurar no app (30 segundos)

1. Abra o app → **Configurações** → seção **Jogo**.
2. Cole o Client ID no campo **"Client ID do Azure"** e clique **Salvar**.
3. Clique **"Entrar com Microsoft"**: o app mostra um código e o link
   https://microsoft.com/link — abra, digite o código, entre com a conta
   que tem o Minecraft.
4. Pronto: sua conta aparece na lista, o avatar surge na barra lateral e o
   jogo passa a abrir autenticado (multiplayer online funciona). A skin
   também fica gerenciável na mesma tela.

## Como o fluxo funciona por dentro (o que você está aprendendo)

```
device code  →  o app pede um código em login.microsoftonline.com/consumers
usuário      →  digita o código em microsoft.com/link e autoriza
MSA token    →  o app troca o device code por access + refresh token
Xbox Live    →  POST user.auth.xboxlive.com/user/authenticate (RpsTicket)
XSTS         →  POST xsts.auth.xboxlive.com/xsts/authorize (RelyingParty
                rp://api.minecraftservices.com/)
Minecraft    →  POST api.minecraftservices.com/authentication/login_with_xbox
                com "XBL3.0 x=<hash>;<token XSTS>"
perfil       →  GET /minecraft/profile (uuid + nome + skins + capas)
```

Tudo isso está implementado em `src-tauri/src/msauth.rs`, incluindo a
renovação automática quando o token do Minecraft expira (24h).
