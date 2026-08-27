type CookieBlock =
  | { type: 'p'; text: string }
  | { type: 'h3'; text: string }
  | { type: 'ul'; items: string[] }

type CookieContent = {
  opening: string[]
  sections: { number: string; title: string; blocks: CookieBlock[] }[]
  faqs: { question: string; answer: string }[]
}

export const cookiesPt: CookieContent = {
  opening: [
    'A presente Política de Cookies explica como a KnitnPrint utiliza cookies e tecnologias semelhantes no seu website, bem como as opções disponíveis aos utilizadores relativamente à sua utilização.',
    'Esta Política deve ser lida em conjunto com a nossa Política de Privacidade, onde encontrará mais informações sobre a forma como tratamos e protegemos os seus dados pessoais.',
  ],
  sections: [
    { number: '01', title: 'O que são cookies?', blocks: [
      { type: 'p', text: 'Os cookies são pequenos ficheiros de informação armazenados no computador, smartphone, tablet ou outro dispositivo utilizado para aceder a um website.' },
      { type: 'p', text: 'Estes ficheiros permitem que o website reconheça o navegador ou dispositivo utilizado, guarde determinadas preferências e recolha informações sobre a navegação e utilização do website.' },
      { type: 'p', text: 'Os cookies podem ser colocados diretamente pelo nosso website ou por serviços prestados por terceiros.' },
    ] },
    { number: '02', title: 'Que tipos de cookies utilizamos?', blocks: [
      { type: 'p', text: 'O website pode utilizar diferentes tipos de cookies consoante a sua finalidade.' },
      { type: 'h3', text: 'Cookies estritamente necessários' },
      { type: 'p', text: 'Estes cookies são essenciais para o correto funcionamento do website e da loja online. Entre outras finalidades, podem ser utilizados para:' },
      { type: 'ul', items: ['Permitir a navegação entre páginas;', 'Manter produtos no carrinho de compras;', 'Processar encomendas;', 'Manter a segurança do website;', 'Prevenir utilizações fraudulentas;', 'Guardar as preferências de consentimento de cookies.'] },
      { type: 'p', text: 'Uma vez que são necessários ao funcionamento do website ou a funcionalidades solicitadas pelo utilizador, a sua utilização geralmente não exige consentimento.' },
      { type: 'h3', text: 'Cookies de análise e desempenho' },
      { type: 'p', text: 'Utilizamos o Google Analytics para obter informações estatísticas sobre a utilização do website. Estas informações podem incluir:' },
      { type: 'ul', items: ['Número de visitantes;', 'Páginas visitadas;', 'Duração aproximada das visitas;', 'Origem do tráfego;', 'Tipo de dispositivo ou navegador utilizado;', 'Interações realizadas no website.'] },
      { type: 'p', text: 'Estas informações ajudam-nos a compreender como a loja online é utilizada, identificar problemas e melhorar o desempenho, o conteúdo e a experiência de navegação.' },
      { type: 'h3', text: 'Cookies de publicidade e marketing' },
      { type: 'p', text: 'Utilizamos o Google Ads para promover os nossos produtos e avaliar o desempenho das campanhas publicitárias. As tecnologias associadas ao Google Ads podem ser utilizadas para:' },
      { type: 'ul', items: ['Medir conversões resultantes de anúncios;', 'Determinar se ocorreu uma compra ou outra ação após a interação com um anúncio;', 'Avaliar o desempenho das campanhas;', 'Limitar ou gerir a apresentação de publicidade;', 'Mostrar publicidade mais relevante quando autorizado.'] },
      { type: 'p', text: 'A Google pode utilizar cookies e identificadores de publicidade, incluindo cookies cujos nomes começam por gcl, para medir ações realizadas após uma interação com publicidade, entre outras finalidades.' },
      { type: 'p', text: 'A utilização de cookies para publicidade, personalização ou medição está sujeita ao consentimento do utilizador sempre que legalmente exigido.' },
    ] },
    { number: '03', title: 'Google Analytics', blocks: [
      { type: 'p', text: 'O nosso website utiliza o Google Analytics, um serviço prestado pela Google, para analisar a utilização do website.' },
      { type: 'p', text: 'Este serviço pode recolher informações sobre a interação dos utilizadores com o website, permitindo produzir estatísticas agregadas e melhorar o funcionamento da loja online.' },
      { type: 'p', text: 'A Google declara que o Google Analytics utiliza cookies para recolher estatísticas de utilização nos websites onde o serviço está implementado.' },
      { type: 'p', text: 'Quando exigido, o Google Analytics será ativado de acordo com as preferências de consentimento selecionadas pelo utilizador.' },
    ] },
    { number: '04', title: 'Google Ads', blocks: [
      { type: 'p', text: 'Utilizamos também o Google Ads para promover os nossos produtos e medir a eficácia das campanhas publicitárias realizadas através dos serviços Google.' },
      { type: 'p', text: 'O Google Ads pode utilizar cookies e tecnologias semelhantes para medir interações com anúncios e conversões concluídas no nosso website.' },
      { type: 'p', text: 'Consoante as escolhas do utilizador, estas tecnologias podem também ser utilizadas para personalizar publicidade.' },
      { type: 'p', text: 'A utilização destas tecnologias está sujeita às preferências de consentimento selecionadas pelo utilizador.' },
    ] },
    { number: '05', title: 'Instagram e serviços Meta', blocks: [
      { type: 'p', text: 'Mantemos uma presença no Instagram, uma plataforma pertencente à Meta.' },
      { type: 'p', text: 'Se o website apenas contiver ligações para a nossa página de Instagram, a plataforma só será acedida quando o utilizador seguir a ligação, ficando a utilização sujeita às políticas da Meta.' },
      { type: 'p', text: 'Se forem utilizados conteúdos incorporados do Instagram, plugins sociais, Meta Pixel ou outras tecnologias Meta, estas podem recolher informações sobre a utilização do website e utilizar cookies ou identificadores semelhantes.' },
      { type: 'p', text: 'Quando estas tecnologias não forem estritamente necessárias, a sua utilização dependerá do consentimento prévio do utilizador.' },
    ] },
    { number: '06', title: 'Cookies de terceiros', blocks: [
      { type: 'p', text: 'Alguns cookies utilizados no website podem ser colocados ou geridos por terceiros, incluindo:' },
      { type: 'ul', items: ['Google Ireland Limited / Google LLC, através do Google Analytics e Google Ads;', 'Meta Platforms, quando são utilizadas funcionalidades, conteúdos incorporados ou tecnologias relacionadas com Instagram ou outros serviços Meta;', 'Outros prestadores tecnológicos necessários ao funcionamento da loja online.'] },
      { type: 'p', text: 'Estas entidades podem tratar informações de acordo com as respetivas políticas de privacidade e cookies.' },
    ] },
    { number: '07', title: 'Duração dos cookies', blocks: [
      { type: 'p', text: 'Os cookies podem ser classificados como:' },
      { type: 'ul', items: ['Cookies de sessão: normalmente eliminados quando o utilizador fecha o navegador;', 'Cookies persistentes: permanecem no dispositivo durante um período definido ou até serem eliminados pelo utilizador.'] },
      { type: 'p', text: 'A sua duração varia consoante o tipo de cookie e o serviço que o coloca.' },
      { type: 'p', text: 'Alguns cookies utilizados pela Google para publicidade podem ter períodos de conservação diferentes consoante a finalidade e a localização do utilizador.' },
    ] },
    { number: '08', title: 'Consentimento e gestão de cookies', blocks: [
      { type: 'p', text: 'Na primeira visita ao website, o utilizador poderá ver um painel ou banner de gestão de cookies. Através desta ferramenta, quando aplicável, poderá:' },
      { type: 'ul', items: ['Aceitar todos os cookies;', 'Recusar cookies não essenciais;', 'Escolher individualmente as categorias de cookies a autorizar;', 'Alterar as preferências posteriormente.'] },
      { type: 'p', text: 'Os cookies que exigem consentimento não serão ativados antes de o utilizador fazer uma escolha válida.' },
      { type: 'p', text: 'O utilizador deve poder retirar o consentimento com a mesma facilidade com que o concedeu.' },
      { type: 'p', text: 'As preferências podem ser alteradas a qualquer momento através da opção “Gerir Cookies”.' },
    ] },
    { number: '09', title: 'Google Consent Mode', blocks: [
      { type: 'p', text: 'O website pode utilizar o Google Consent Mode para comunicar à Google as escolhas de consentimento do utilizador.' },
      { type: 'p', text: 'Este sistema permite que as etiquetas do Google Analytics e Google Ads ajustem o seu comportamento às preferências de privacidade selecionadas.' },
      { type: 'p', text: 'O Consent Mode não substitui o banner de cookies nem o mecanismo utilizado para solicitar o consentimento.' },
    ] },
    { number: '10', title: 'Gestão de cookies através do navegador', blocks: [
      { type: 'p', text: 'O utilizador pode também configurar o navegador para bloquear ou eliminar cookies.' },
      { type: 'p', text: 'Estas definições encontram-se normalmente nas opções de privacidade ou segurança do navegador.' },
      { type: 'p', text: 'A desativação de determinados cookies pode afetar algumas funcionalidades do website, sobretudo quando são necessários ao funcionamento da loja online.' },
    ] },
    { number: '11', title: 'Dados pessoais', blocks: [
      { type: 'p', text: 'Algumas informações recolhidas através de cookies ou tecnologias semelhantes podem constituir dados pessoais, incluindo:' },
      { type: 'ul', items: ['Endereço IP;', 'Identificadores online;', 'Informações do navegador ou dispositivo;', 'Dados de interação com o website;', 'Informações relacionadas com campanhas publicitárias.'] },
      { type: 'p', text: 'Sempre que sejam tratados dados pessoais, o tratamento será realizado de acordo com a legislação aplicável e a nossa Política de Privacidade.' },
    ] },
    { number: '12', title: 'Alterações à Política de Cookies', blocks: [
      { type: 'p', text: 'A KnitnPrint pode atualizar esta Política de Cookies sempre que necessário, incluindo em resultado de alterações legislativas, tecnológicas ou dos serviços utilizados no website.' },
      { type: 'p', text: 'Recomendamos a consulta periódica desta página.' },
      { type: 'p', text: 'A data da atualização mais recente será indicada no início do documento.' },
    ] },
    { number: '13', title: 'Contactos', blocks: [
      { type: 'p', text: 'Para qualquer questão sobre esta Política de Cookies ou a proteção dos seus dados pessoais, contacte a KnitnPrint através de [email].' },
    ] },
  ],
  faqs: [
    { question: 'O que são cookies?', answer: 'São pequenos ficheiros guardados pelo website no navegador ou dispositivo para suportar funcionalidades, recordar preferências e compreender a utilização do website.' },
    { question: 'Que cookies utiliza a KnitnPrint?', answer: 'O website pode utilizar cookies necessários, de preferências, análise e publicidade, conforme descrito nesta Política de Cookies.' },
    { question: 'Posso recusar cookies opcionais?', answer: 'Sim. Quando existir um painel de consentimento, os cookies opcionais devem permanecer inativos até fazer uma escolha válida.' },
    { question: 'Como posso alterar as preferências?', answer: 'Utilize a opção Gerir Cookies, quando disponível, ou reveja os controlos de privacidade e cookies do seu navegador.' },
    { question: 'O website funciona se recusar cookies?', answer: 'Os cookies necessários suportam funções essenciais. Recusar cookies opcionais não deverá impedir essas funções, embora algumas funcionalidades adicionais possam ser afetadas.' },
  ],
}

export const cookiesEs: CookieContent = {
  opening: [
    'Esta Política de cookies explica cómo KnitnPrint utiliza cookies y tecnologías similares en su sitio web, así como las opciones disponibles para los usuarios.',
    'Esta Política debe leerse junto con nuestra Política de privacidad, donde encontrarás más información sobre cómo tratamos y protegemos tus datos personales.',
  ],
  sections: [
    { number: '01', title: '¿Qué son las cookies?', blocks: [
      { type: 'p', text: 'Las cookies son pequeños archivos de información almacenados en el ordenador, teléfono, tableta u otro dispositivo utilizado para acceder a un sitio web.' },
      { type: 'p', text: 'Permiten reconocer el navegador o dispositivo, guardar preferencias y recoger información sobre la navegación y el uso del sitio web.' },
      { type: 'p', text: 'Las cookies pueden ser instaladas directamente por nuestro sitio web o por servicios de terceros.' },
    ] },
    { number: '02', title: '¿Qué tipos de cookies utilizamos?', blocks: [
      { type: 'p', text: 'El sitio web puede utilizar distintos tipos de cookies según su finalidad.' },
      { type: 'h3', text: 'Cookies estrictamente necesarias' },
      { type: 'p', text: 'Son esenciales para el correcto funcionamiento del sitio web y la tienda online. Pueden utilizarse para:' },
      { type: 'ul', items: ['Permitir la navegación entre páginas;', 'Mantener productos en la cesta;', 'Procesar pedidos;', 'Mantener la seguridad del sitio;', 'Prevenir usos fraudulentos;', 'Guardar las preferencias de consentimiento.'] },
      { type: 'p', text: 'Como son necesarias para el funcionamiento o para funciones solicitadas, su uso generalmente no requiere consentimiento.' },
      { type: 'h3', text: 'Cookies de análisis y rendimiento' },
      { type: 'p', text: 'Utilizamos Google Analytics para obtener información estadística sobre el uso del sitio web, que puede incluir:' },
      { type: 'ul', items: ['Número de visitantes;', 'Páginas visitadas;', 'Duración aproximada de las visitas;', 'Origen del tráfico;', 'Tipo de dispositivo o navegador;', 'Interacciones realizadas.'] },
      { type: 'p', text: 'Esta información nos ayuda a comprender el uso, identificar problemas y mejorar el rendimiento, el contenido y la experiencia de navegación.' },
      { type: 'h3', text: 'Cookies de publicidad y marketing' },
      { type: 'p', text: 'Utilizamos Google Ads para promocionar productos y evaluar nuestras campañas. Sus tecnologías pueden utilizarse para:' },
      { type: 'ul', items: ['Medir conversiones procedentes de anuncios;', 'Determinar si se produjo una compra u otra acción tras interactuar con un anuncio;', 'Evaluar el rendimiento de las campañas;', 'Limitar o gestionar la publicidad;', 'Mostrar publicidad más relevante cuando esté autorizado.'] },
      { type: 'p', text: 'Google puede utilizar cookies e identificadores publicitarios, incluidas cookies cuyos nombres empiezan por gcl, para medir acciones posteriores a una interacción publicitaria, entre otras finalidades.' },
      { type: 'p', text: 'El uso de cookies para publicidad, personalización o medición está sujeto al consentimiento cuando la ley lo exija.' },
    ] },
    { number: '03', title: 'Google Analytics', blocks: [
      { type: 'p', text: 'Nuestro sitio web utiliza Google Analytics, un servicio de Google, para analizar su uso.' },
      { type: 'p', text: 'Este servicio puede recoger información sobre la interacción de los usuarios para producir estadísticas agregadas y mejorar la tienda online.' },
      { type: 'p', text: 'Google indica que Google Analytics utiliza cookies para recoger estadísticas de uso en los sitios donde está implementado.' },
      { type: 'p', text: 'Cuando sea necesario, Google Analytics se activará según las preferencias de consentimiento seleccionadas.' },
    ] },
    { number: '04', title: 'Google Ads', blocks: [
      { type: 'p', text: 'También utilizamos Google Ads para promocionar productos y medir la eficacia de campañas realizadas mediante servicios de Google.' },
      { type: 'p', text: 'Google Ads puede utilizar cookies y tecnologías similares para medir interacciones con anuncios y conversiones en nuestro sitio.' },
      { type: 'p', text: 'Según las elecciones del usuario, también pueden utilizarse para personalizar la publicidad.' },
      { type: 'p', text: 'El uso de estas tecnologías está sujeto a las preferencias de consentimiento seleccionadas.' },
    ] },
    { number: '05', title: 'Instagram y servicios de Meta', blocks: [
      { type: 'p', text: 'Mantenemos una presencia en Instagram, plataforma propiedad de Meta.' },
      { type: 'p', text: 'Si el sitio solo contiene enlaces a Instagram, la plataforma se abre únicamente cuando el usuario sigue el enlace y su uso queda sujeto a las políticas de Meta.' },
      { type: 'p', text: 'Si se utilizan contenidos integrados, plugins sociales, Meta Pixel u otras tecnologías de Meta, pueden recoger información sobre el uso y emplear cookies o identificadores similares.' },
      { type: 'p', text: 'Cuando no sean estrictamente necesarias, su uso dependerá del consentimiento previo.' },
    ] },
    { number: '06', title: 'Cookies de terceros', blocks: [
      { type: 'p', text: 'Algunas cookies pueden ser instaladas o gestionadas por terceros, incluidos:' },
      { type: 'ul', items: ['Google Ireland Limited / Google LLC, mediante Google Analytics y Google Ads;', 'Meta Platforms, cuando se utilicen funciones, contenidos o tecnologías de Instagram u otros servicios Meta;', 'Otros proveedores tecnológicos necesarios para la tienda online.'] },
      { type: 'p', text: 'Estas entidades pueden tratar información según sus propias políticas de privacidad y cookies.' },
    ] },
    { number: '07', title: 'Duración de las cookies', blocks: [
      { type: 'p', text: 'Las cookies pueden ser:' },
      { type: 'ul', items: ['Cookies de sesión: normalmente se eliminan al cerrar el navegador;', 'Cookies persistentes: permanecen durante un periodo definido o hasta que el usuario las elimina.'] },
      { type: 'p', text: 'Su duración varía según el tipo de cookie y el servicio que la instala.' },
      { type: 'p', text: 'Algunas cookies publicitarias de Google pueden tener periodos distintos según su finalidad y la ubicación del usuario.' },
    ] },
    { number: '08', title: 'Consentimiento y gestión de cookies', blocks: [
      { type: 'p', text: 'En la primera visita puede aparecer un panel o banner de gestión. Cuando proceda, permite:' },
      { type: 'ul', items: ['Aceptar todas las cookies;', 'Rechazar las no esenciales;', 'Elegir individualmente las categorías autorizadas;', 'Cambiar las preferencias posteriormente.'] },
      { type: 'p', text: 'Las cookies que requieren consentimiento no se activarán antes de una elección válida.' },
      { type: 'p', text: 'El consentimiento debe poder retirarse con la misma facilidad con que se otorgó.' },
      { type: 'p', text: 'Las preferencias pueden cambiarse mediante la opción “Gestionar cookies”.' },
    ] },
    { number: '09', title: 'Google Consent Mode', blocks: [
      { type: 'p', text: 'El sitio puede utilizar Google Consent Mode para comunicar a Google las elecciones de consentimiento.' },
      { type: 'p', text: 'Este sistema permite que las etiquetas de Google Analytics y Google Ads adapten su comportamiento a las preferencias seleccionadas.' },
      { type: 'p', text: 'Consent Mode no sustituye al banner ni al mecanismo utilizado para solicitar consentimiento.' },
    ] },
    { number: '10', title: 'Gestión desde el navegador', blocks: [
      { type: 'p', text: 'También puedes configurar el navegador para bloquear o eliminar cookies.' },
      { type: 'p', text: 'Estas opciones suelen estar en la configuración de privacidad o seguridad.' },
      { type: 'p', text: 'Desactivar determinadas cookies puede afectar a funciones del sitio, especialmente cuando son necesarias para la tienda online.' },
    ] },
    { number: '11', title: 'Datos personales', blocks: [
      { type: 'p', text: 'Parte de la información recogida mediante cookies puede constituir datos personales, incluidos:' },
      { type: 'ul', items: ['Dirección IP;', 'Identificadores online;', 'Información del navegador o dispositivo;', 'Datos de interacción con el sitio;', 'Información relacionada con campañas publicitarias.'] },
      { type: 'p', text: 'Cuando se traten datos personales, se hará conforme a la legislación aplicable y a nuestra Política de privacidad.' },
    ] },
    { number: '12', title: 'Cambios en la Política de cookies', blocks: [
      { type: 'p', text: 'KnitnPrint puede actualizar esta Política cuando sea necesario por cambios legales, tecnológicos o en los servicios utilizados.' },
      { type: 'p', text: 'Recomendamos consultar esta página periódicamente.' },
      { type: 'p', text: 'La fecha de la última actualización se mostrará al principio del documento.' },
    ] },
    { number: '13', title: 'Contacto', blocks: [
      { type: 'p', text: 'Para preguntas sobre esta Política de cookies o la protección de tus datos personales, contacta con KnitnPrint en [email].' },
    ] },
  ],
  faqs: [
    { question: '¿Qué son las cookies?', answer: 'Son pequeños archivos guardados por un sitio web en el navegador o dispositivo para permitir funciones, recordar preferencias y comprender su uso.' },
    { question: '¿Qué cookies utiliza KnitnPrint?', answer: 'El sitio puede utilizar cookies necesarias, de preferencias, análisis y publicidad, como se describe en esta Política.' },
    { question: '¿Puedo rechazar las cookies opcionales?', answer: 'Sí. Cuando exista un panel de consentimiento, las cookies opcionales deben permanecer inactivas hasta que hagas una elección válida.' },
    { question: '¿Cómo puedo cambiar mis preferencias?', answer: 'Utiliza la opción Gestionar cookies cuando esté disponible o revisa los controles de privacidad y cookies del navegador.' },
    { question: '¿Funcionará el sitio si rechazo cookies?', answer: 'Las cookies necesarias permiten funciones esenciales. Rechazar las opcionales no debería impedirlas, aunque algunas funciones adicionales podrían verse afectadas.' },
  ],
}
