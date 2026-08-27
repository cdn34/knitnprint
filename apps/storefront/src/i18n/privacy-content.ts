import type { LegalSection } from './terms-content'

type PrivacyContent = {
  opening: string[]
  sections: LegalSection[]
  faqs: { question: string; answer: string }[]
}

export const privacyPt: PrivacyContent = {
  opening: [
    'A presente Política de Privacidade descreve como a KnitnPrint, responsável pelo website [site], recolhe, utiliza e protege os dados pessoais dos utilizadores quando visitam o website ou efetuam compras através da loja online.',
    'Ao utilizar este website, aceita as práticas descritas na presente Política de Privacidade.',
  ],
  sections: [
    { number: '02', title: 'Dados que recolhemos', blocks: [
      { type: 'p', text: 'Quando visita a nossa loja online, recolhemos determinadas informações para podermos melhorar os nossos serviços e responder aos clientes de forma mais eficaz.' },
      { type: 'p', text: 'As informações recolhidas incluem:' },
      { type: 'ul', items: ['Dados de contacto: nome, morada, número de telefone e endereço de email;', 'Informações da encomenda: nome, morada de entrega, morada de faturação, confirmação de pagamento, email e número de telefone;', 'Informações da conta: nome de utilizador e palavra-passe.'] },
      { type: 'p', text: 'Podemos também obter informações sobre si através de terceiros. Por exemplo, os prestadores de pagamento podem fornecer os dados necessários para confirmar que uma transação cumpre a nossa política de compras.' },
      { type: 'p', text: 'Podemos recolher informações sobre a utilização dos nossos serviços através de cookies. Estas podem incluir a forma como acede ao website, informações do navegador e da ligação de rede, endereço IP e outros dados sobre as suas interações com os serviços.' },
    ] },
    { number: '03', title: 'Finalidades e tratamento dos dados pessoais', blocks: [
      { type: 'p', text: 'Os dados pessoais recolhidos através do website podem ser utilizados para:' },
      { type: 'ul', items: ['Processar, gerir e acompanhar encomendas;', 'Processar os respetivos pagamentos;', 'Organizar e acompanhar a expedição e entrega das encomendas;', 'Emitir faturas, recibos e confirmações de compra;', 'Prestar apoio e serviço ao cliente;', 'Prevenir e detetar possíveis fraudes ou outras atividades ilícitas;', 'Melhorar o funcionamento, a segurança e o desempenho do website;', 'Enviar comunicações promocionais, novidades ou informações sobre produtos quando o utilizador tiver dado consentimento prévio.'] },
    ] },
    { number: '04', title: 'Partilha de dados pessoais', blocks: [
      { type: 'p', text: 'Os dados pessoais podem ser partilhados com terceiros que prestem serviços essenciais ao funcionamento da loja online, incluindo:' },
      { type: 'ul', items: ['Prestadores de serviços de pagamento;', 'Transportadoras responsáveis pela entrega das encomendas;', 'Plataformas e serviços de análise de tráfego e utilização do website.'] },
      { type: 'p', text: 'Podemos utilizar serviços de análise, como o Google Analytics, para compreender como os utilizadores interagem com o website e melhorar a navegação, o desempenho e a experiência de utilização.' },
      { type: 'p', text: 'Estas entidades apenas terão acesso aos dados estritamente necessários à prestação dos serviços contratados e deverão cumprir a legislação aplicável em matéria de proteção de dados pessoais.' },
      { type: 'p', text: 'Os dados podem ainda ser divulgados quando necessário para:' },
      { type: 'ul', items: ['Cumprir obrigações legais ou regulamentares;', 'Responder a pedidos de autoridades ou entidades competentes;', 'Defender e proteger os direitos e interesses legítimos da empresa.'] },
    ] },
    { number: '05', title: 'Publicidade e marketing', blocks: [
      { type: 'p', text: 'Podemos utilizar os dados recolhidos para apresentar anúncios ou comunicações de marketing que possam ser do seu interesse.' },
      { type: 'p', text: 'Para este efeito, podemos utilizar serviços de publicidade de plataformas como:' },
      { type: 'ul', items: ['Facebook;', 'Google;', 'Instagram.'] },
      { type: 'p', text: 'Pode cancelar a subscrição de comunicações de marketing a qualquer momento.' },
    ] },
    { number: '06', title: 'Direitos dos titulares dos dados', blocks: [
      { type: 'p', text: 'Nos termos do Regulamento Geral sobre a Proteção de Dados (RGPD), os utilizadores têm o direito de:' },
      { type: 'ul', items: ['Aceder aos seus dados pessoais;', 'Solicitar a correção de dados inexatos;', 'Solicitar a eliminação dos seus dados;', 'Limitar ou opor-se ao tratamento dos seus dados;', 'Solicitar a portabilidade dos dados.'] },
    ] },
    { number: '07', title: 'Transferências internacionais de dados', blocks: [
      { type: 'p', text: 'Alguns serviços utilizados pelo website podem envolver a transferência de dados para fora do Espaço Económico Europeu, incluindo para países como os Estados Unidos ou o Canadá.' },
      { type: 'p', text: 'Sempre que tal aconteça, são adotadas garantias adequadas para proteger os dados pessoais em conformidade com o RGPD.' },
    ] },
    { number: '08', title: 'Conservação dos dados', blocks: [
      { type: 'p', text: 'Os dados pessoais serão conservados apenas durante o período necessário para cumprir as finalidades para as quais foram recolhidos, incluindo obrigações legais, fiscais e contabilísticas.' },
      { type: 'p', text: 'Quando efetua uma encomenda, os dados relacionados podem ser conservados para fins legais e fiscais, salvo se solicitar expressamente a sua eliminação, quando aplicável.' },
    ] },
    { number: '09', title: 'Alterações à Política de Privacidade', blocks: [
      { type: 'p', text: 'Reservamo-nos o direito de atualizar esta Política de Privacidade sempre que necessário para refletir alterações legais, técnicas ou operacionais.' },
      { type: 'p', text: 'Quaisquer alterações serão publicadas nesta página.' },
    ] },
    { number: '10', title: 'Contactos', blocks: [
      { type: 'p', text: 'Se tiver alguma questão sobre esta Política de Privacidade ou sobre a forma como tratamos os seus dados pessoais, contacte-nos através de [email].' },
    ] },
  ],
  faqs: [
    { question: 'Que dados pessoais recolhem?', answer: 'Consoante a forma como utiliza o website, podemos recolher dados de contacto, encomenda, entrega, pagamento e utilização do website.' },
    { question: 'Porque precisam das minhas informações?', answer: 'Utilizamos as informações necessárias para processar encomendas, prestar apoio, cumprir obrigações legais, melhorar os serviços e, quando permitido, comunicar consigo.' },
    { question: 'Partilham os meus dados com outras entidades?', answer: 'Apenas quando necessário, por exemplo com prestadores de pagamento, entrega, alojamento ou serviços profissionais, ou quando a divulgação seja legalmente exigida.' },
    { question: 'Como posso exercer os meus direitos de proteção de dados?', answer: 'Pode solicitar acesso, correção, eliminação, limitação, oposição ou portabilidade, quando aplicável ao abrigo da legislação de proteção de dados.' },
    { question: 'Como posso contactar-vos sobre os meus dados?', answer: 'Envie a sua questão ou pedido de privacidade para support@knitnprint.com.' },
  ],
}

export const privacyEs: PrivacyContent = {
  opening: [
    'Esta Política de privacidad describe cómo KnitnPrint, responsable de [site], recoge, utiliza y protege los datos personales de los usuarios cuando visitan el sitio web o compran en la tienda online.',
    'Al utilizar este sitio web, aceptas las prácticas descritas en esta Política de privacidad.',
  ],
  sections: [
    { number: '02', title: 'Datos que recogemos', blocks: [
      { type: 'p', text: 'Cuando visitas nuestra tienda online, recogemos determinada información para mejorar nuestros servicios y atender a los clientes de manera más eficaz.' },
      { type: 'p', text: 'La información recogida incluye:' },
      { type: 'ul', items: ['Datos de contacto: nombre, dirección, teléfono y correo electrónico;', 'Información del pedido: nombre, dirección de entrega, dirección de facturación, confirmación de pago, correo electrónico y teléfono;', 'Información de la cuenta: nombre de usuario y contraseña.'] },
      { type: 'p', text: 'También podemos obtener información sobre ti de terceros. Por ejemplo, los proveedores de pagos pueden facilitar los datos necesarios para confirmar que una transacción cumple nuestra política de compra.' },
      { type: 'p', text: 'Podemos recoger información sobre el uso de nuestros servicios mediante cookies, incluida la forma de acceso, el navegador y la conexión de red, la dirección IP y otros datos sobre tus interacciones.' },
    ] },
    { number: '03', title: 'Finalidades y tratamiento de datos personales', blocks: [
      { type: 'p', text: 'Los datos personales recogidos a través del sitio web pueden utilizarse para:' },
      { type: 'ul', items: ['Procesar, gestionar y seguir pedidos;', 'Procesar los pagos correspondientes;', 'Organizar y seguir el envío y la entrega;', 'Emitir facturas, recibos y confirmaciones de compra;', 'Prestar atención y soporte al cliente;', 'Prevenir y detectar posibles fraudes u otras actividades ilícitas;', 'Mejorar el funcionamiento, la seguridad y el rendimiento del sitio web;', 'Enviar comunicaciones promocionales, novedades o información sobre productos cuando exista consentimiento previo.'] },
    ] },
    { number: '04', title: 'Comunicación de datos personales', blocks: [
      { type: 'p', text: 'Los datos personales pueden compartirse con terceros que prestan servicios esenciales para la tienda online, incluidos:' },
      { type: 'ul', items: ['Proveedores de servicios de pago;', 'Transportistas responsables de la entrega;', 'Plataformas y servicios de análisis de tráfico y uso del sitio web.'] },
      { type: 'p', text: 'Podemos utilizar servicios de análisis como Google Analytics para comprender cómo interactúan los usuarios con el sitio web y mejorar la navegación, el rendimiento y la experiencia.' },
      { type: 'p', text: 'Estas entidades solo tendrán acceso a los datos estrictamente necesarios para prestar los servicios contratados y deberán cumplir la legislación aplicable sobre protección de datos.' },
      { type: 'p', text: 'Los datos también podrán divulgarse cuando sea necesario para:' },
      { type: 'ul', items: ['Cumplir obligaciones legales o reglamentarias;', 'Responder a solicitudes de autoridades u organismos competentes;', 'Defender y proteger los derechos e intereses legítimos de la empresa.'] },
    ] },
    { number: '05', title: 'Publicidad y marketing', blocks: [
      { type: 'p', text: 'Podemos utilizar los datos recogidos para mostrar anuncios o comunicaciones de marketing que puedan interesarte.' },
      { type: 'p', text: 'Para ello podemos utilizar servicios publicitarios de plataformas como:' },
      { type: 'ul', items: ['Facebook;', 'Google;', 'Instagram.'] },
      { type: 'p', text: 'Puedes cancelar la suscripción a las comunicaciones de marketing en cualquier momento.' },
    ] },
    { number: '06', title: 'Derechos de los interesados', blocks: [
      { type: 'p', text: 'De acuerdo con el Reglamento General de Protección de Datos (RGPD), los usuarios tienen derecho a:' },
      { type: 'ul', items: ['Acceder a sus datos personales;', 'Solicitar la corrección de datos inexactos;', 'Solicitar la eliminación de sus datos;', 'Limitar u oponerse al tratamiento;', 'Solicitar la portabilidad de los datos.'] },
    ] },
    { number: '07', title: 'Transferencias internacionales de datos', blocks: [
      { type: 'p', text: 'Algunos servicios utilizados por el sitio web pueden implicar transferencias fuera del Espacio Económico Europeo, incluidos países como Estados Unidos o Canadá.' },
      { type: 'p', text: 'Cuando esto ocurra, se adoptarán garantías adecuadas para proteger los datos personales de conformidad con el RGPD.' },
    ] },
    { number: '08', title: 'Conservación de los datos', blocks: [
      { type: 'p', text: 'Los datos personales solo se conservarán durante el tiempo necesario para cumplir las finalidades para las que fueron recogidos, incluidas las obligaciones legales, fiscales y contables.' },
      { type: 'p', text: 'Cuando realizas un pedido, los datos relacionados pueden conservarse con fines legales y fiscales, salvo que solicites expresamente su eliminación cuando proceda.' },
    ] },
    { number: '09', title: 'Cambios en la Política de privacidad', blocks: [
      { type: 'p', text: 'Nos reservamos el derecho a actualizar esta Política de privacidad cuando sea necesario para reflejar cambios legales, técnicos u operativos.' },
      { type: 'p', text: 'Cualquier cambio se publicará en esta página.' },
    ] },
    { number: '10', title: 'Contacto', blocks: [
      { type: 'p', text: 'Si tienes preguntas sobre esta Política de privacidad o sobre cómo tratamos tus datos personales, contacta con nosotros en [email].' },
    ] },
  ],
  faqs: [
    { question: '¿Qué datos personales recogéis?', answer: 'Según cómo utilices el sitio web, podemos recoger datos de contacto, pedido, entrega, pago y uso del sitio.' },
    { question: '¿Por qué necesitáis mi información?', answer: 'Utilizamos la información necesaria para procesar pedidos, prestar soporte, cumplir obligaciones legales, mejorar los servicios y comunicarnos contigo cuando esté permitido.' },
    { question: '¿Compartís mis datos con otras organizaciones?', answer: 'Solo cuando sea necesario, por ejemplo con proveedores de pago, entrega, alojamiento o servicios profesionales, o cuando la ley exija su divulgación.' },
    { question: '¿Cómo puedo ejercer mis derechos de protección de datos?', answer: 'Puedes solicitar acceso, corrección, eliminación, limitación, oposición o portabilidad cuando proceda según la legislación de protección de datos.' },
    { question: '¿Cómo puedo contactar sobre mis datos?', answer: 'Envía tu pregunta o solicitud de privacidad a support@knitnprint.com.' },
  ],
}
