import type { LegalSection } from './terms-content'

type ReturnsContent = {
  sections: LegalSection[]
  faqs: { question: string; answer: string }[]
}

export const returnsPt: ReturnsContent = {
  sections: [
    { number: '01', title: 'Âmbito', blocks: [
      { type: 'p', text: 'A presente Política de Trocas, Devoluções e Reembolsos aplica-se a todas as compras efetuadas na KnitnPrint e estabelece as condições em que os clientes podem solicitar a devolução ou troca de um produto, bem como o respetivo reembolso.' },
    ] },
    { number: '02', title: 'Produtos não personalizados', blocks: [
      { type: 'p', text: '2.1 Direito de livre resolução — Nos termos do Decreto-Lei n.º 24/2014, o cliente pode exercer o direito de livre resolução no prazo de 14 dias a contar da data em que recebe a encomenda, sem necessidade de indicar qualquer motivo.' },
      { type: 'p', text: '2.2 Condições da devolução — Para que a devolução seja aceite, o produto deve:' },
      { type: 'ul', items: ['Estar em perfeito estado e não apresentar sinais de utilização;', 'Ser devolvido na embalagem original, juntamente com todos os componentes e acessórios;', 'Ser acompanhado do respetivo comprovativo de compra.'] },
      { type: 'p', text: '2.3 Custos da devolução — Os custos associados à devolução do produto são da responsabilidade do cliente, salvo quando a devolução resulte de um erro imputável à KnitnPrint ou de um defeito do produto.' },
      { type: 'p', text: '2.4 Reembolso — Depois de o produto devolvido ser recebido e verificado, o reembolso será processado no prazo máximo de 14 dias.' },
      { type: 'p', text: 'O valor será reembolsado através do mesmo método de pagamento utilizado na compra, salvo acordo expresso em contrário.' },
    ] },
    { number: '03', title: 'Produtos personalizados', blocks: [
      { type: 'p', text: '3.1 Exclusão do direito de livre resolução — Nos termos da legislação aplicável, os produtos personalizados, produzidos segundo especificações do cliente ou claramente adaptados às suas necessidades não podem ser devolvidos ao abrigo do direito de livre resolução.' },
      { type: 'p', text: 'Esta exclusão não afeta os direitos do cliente quando o produto apresente um defeito, erro ou falta de conformidade.' },
      { type: 'p', text: '3.2 Situações elegíveis — A KnitnPrint aceitará reclamações relativas a produtos personalizados nas seguintes situações:' },
      { type: 'ul', items: ['Defeito de fabrico;', 'Erro de personalização imputável à KnitnPrint, incluindo diferenças face à maquete aprovada;', 'Danos ocorridos durante o transporte.'] },
      { type: 'p', text: '3.3 Procedimento — O cliente deve comunicar o problema no prazo máximo de cinco dias úteis após a receção da encomenda, enviando fotografias do produto e uma descrição detalhada da situação.' },
      { type: 'p', text: 'Depois de analisar o pedido, a KnitnPrint poderá propor, consoante as circunstâncias:' },
      { type: 'ul', items: ['A substituição do produto;', 'Um reembolso parcial ou total;', 'Crédito em loja.'] },
    ] },
    { number: '04', title: 'Produtos danificados ou incorretos', blocks: [
      { type: 'p', text: 'Se o cliente receber um produto danificado ou diferente do encomendado, deverá contactar a KnitnPrint no prazo máximo de cinco dias úteis após a receção.' },
      { type: 'p', text: 'O pedido deve incluir:' },
      { type: 'ul', items: ['Fotografias do produto recebido;', 'O número da encomenda;', 'Uma descrição detalhada do problema.'] },
      { type: 'p', text: 'Quando se confirme um dano, erro ou falta de conformidade imputável à KnitnPrint, esta suportará os custos da devolução e, consoante o caso, substituirá o produto ou emitirá o respetivo reembolso.' },
    ] },
    { number: '05', title: 'Como solicitar uma devolução', blocks: [
      { type: 'p', text: 'Para iniciar o processo de devolução, o cliente deve:' },
      { type: 'ol', items: ['Contactar a KnitnPrint através de support@knitnprint.com;', 'Indicar o número da encomenda e o motivo da devolução;', 'Aguardar as instruções necessárias antes de enviar o produto.'] },
      { type: 'p', text: 'Os produtos não devem ser enviados antes de serem fornecidas as respetivas instruções de devolução.' },
    ] },
    { number: '06', title: 'Contactos', blocks: [
      { type: 'p', text: 'Para qualquer questão sobre trocas, devoluções ou reembolsos, contacte a KnitnPrint através de support@knitnprint.com.' },
    ] },
    { number: '07', title: 'Alterações a esta Política', blocks: [
      { type: 'p', text: 'A KnitnPrint reserva-se o direito de atualizar ou alterar esta Política a qualquer momento. As alterações serão publicadas no website e produzirão efeitos a partir da data da sua publicação.' },
    ] },
  ],
  faqs: [
    { question: 'Posso devolver um produto personalizado?', answer: 'Os produtos personalizados não podem ser devolvidos ao abrigo do direito de livre resolução, mas os seus direitos mantêm-se quando o artigo tem um defeito, chega danificado ou não corresponde ao acordado.' },
    { question: 'O que devo fazer se a encomenda chegar danificada?', answer: 'Contacte-nos no prazo de cinco dias úteis, indicando o número da encomenda e enviando uma descrição do problema e fotografias nítidas do produto e da embalagem.' },
    { question: 'Como inicio uma devolução?', answer: 'Envie um email para support@knitnprint.com com o número da encomenda e o motivo da devolução e aguarde as nossas instruções antes de enviar o produto.' },
    { question: 'Quem paga os custos da devolução?', answer: 'Normalmente, os custos são suportados pelo cliente, exceto quando a devolução resulta de um erro da KnitnPrint ou de um defeito confirmado do produto.' },
    { question: 'Quando receberei o reembolso?', answer: 'Depois de uma devolução elegível ser recebida e verificada, o reembolso será processado no prazo máximo de 14 dias.' },
  ],
}

export const returnsEs: ReturnsContent = {
  sections: [
    { number: '01', title: 'Ámbito', blocks: [
      { type: 'p', text: 'Esta Política de Cambios, Devoluciones y Reembolsos se aplica a todas las compras realizadas en KnitnPrint y establece las condiciones para solicitar la devolución o el cambio de un producto, así como el correspondiente reembolso.' },
    ] },
    { number: '02', title: 'Productos no personalizados', blocks: [
      { type: 'p', text: '2.1 Derecho de desistimiento — De conformidad con el Decreto-Ley portugués n.º 24/2014, el cliente puede ejercer su derecho de desistimiento dentro de los 14 días siguientes a la recepción del pedido, sin necesidad de indicar el motivo.' },
      { type: 'p', text: '2.2 Condiciones de devolución — Para que una devolución sea aceptada, el producto debe:' },
      { type: 'ul', items: ['Estar en perfecto estado y no mostrar señales de uso;', 'Devolverse en su embalaje original, con todos sus componentes y accesorios;', 'Ir acompañado del correspondiente comprobante de compra.'] },
      { type: 'p', text: '2.3 Gastos de devolución — Los gastos asociados a la devolución son responsabilidad del cliente, salvo que esta se deba a un error atribuible a KnitnPrint o a un defecto del producto.' },
      { type: 'p', text: '2.4 Reembolso — Una vez recibido e inspeccionado el producto devuelto, el reembolso se procesará en un plazo máximo de 14 días.' },
      { type: 'p', text: 'El importe se reembolsará mediante el mismo método de pago utilizado en la compra, salvo acuerdo expreso en contrario.' },
    ] },
    { number: '03', title: 'Productos personalizados', blocks: [
      { type: 'p', text: '3.1 Exclusión del derecho de desistimiento — De acuerdo con la legislación aplicable, los productos personalizados, fabricados conforme a las especificaciones del cliente o claramente adaptados a sus necesidades no pueden devolverse al amparo del derecho de desistimiento.' },
      { type: 'p', text: 'Esta exclusión no afecta a los derechos del cliente cuando el producto presenta un defecto, error o falta de conformidad.' },
      { type: 'p', text: '3.2 Situaciones admisibles — KnitnPrint aceptará reclamaciones sobre productos personalizados en los siguientes casos:' },
      { type: 'ul', items: ['Defecto de fabricación;', 'Error de personalización atribuible a KnitnPrint, incluidas diferencias respecto al diseño aprobado;', 'Daños producidos durante el transporte.'] },
      { type: 'p', text: '3.3 Procedimiento — El cliente debe comunicar el problema en un plazo máximo de cinco días laborables desde la recepción, aportando fotografías del producto y una descripción detallada.' },
      { type: 'p', text: 'Tras revisar la solicitud, KnitnPrint podrá proponer, según las circunstancias:' },
      { type: 'ul', items: ['La sustitución del producto;', 'Un reembolso parcial o total;', 'Crédito en tienda.'] },
    ] },
    { number: '04', title: 'Productos dañados o incorrectos', blocks: [
      { type: 'p', text: 'Si el cliente recibe un producto dañado o distinto del solicitado, deberá contactar con KnitnPrint en un plazo máximo de cinco días laborables desde su recepción.' },
      { type: 'p', text: 'La solicitud debe incluir:' },
      { type: 'ul', items: ['Fotografías del producto recibido;', 'El número de pedido;', 'Una descripción detallada del problema.'] },
      { type: 'p', text: 'Cuando se confirme un daño, error o falta de conformidad atribuible a KnitnPrint, esta asumirá los gastos de devolución y, según corresponda, sustituirá el producto o emitirá el reembolso.' },
    ] },
    { number: '05', title: 'Cómo solicitar una devolución', blocks: [
      { type: 'p', text: 'Para iniciar el proceso de devolución, el cliente debe:' },
      { type: 'ol', items: ['Contactar con KnitnPrint en support@knitnprint.com;', 'Indicar el número de pedido y el motivo de la devolución;', 'Esperar las instrucciones necesarias antes de enviar el producto.'] },
      { type: 'p', text: 'Los productos no deben enviarse antes de recibir las instrucciones de devolución correspondientes.' },
    ] },
    { number: '06', title: 'Contacto', blocks: [
      { type: 'p', text: 'Para cualquier pregunta sobre cambios, devoluciones o reembolsos, contacta con KnitnPrint en support@knitnprint.com.' },
    ] },
    { number: '07', title: 'Cambios en esta Política', blocks: [
      { type: 'p', text: 'KnitnPrint se reserva el derecho a actualizar o modificar esta Política en cualquier momento. Los cambios se publicarán en el sitio web y serán efectivos desde la fecha de publicación.' },
    ] },
  ],
  faqs: [
    { question: '¿Puedo devolver un producto personalizado?', answer: 'Los productos personalizados no pueden devolverse al amparo del derecho de desistimiento, pero tus derechos se mantienen si el artículo es defectuoso, llega dañado o no coincide con lo acordado.' },
    { question: '¿Qué debo hacer si mi pedido llega dañado?', answer: 'Contacta con nosotros en un plazo de cinco días laborables con el número de pedido, una descripción del problema y fotografías claras del producto y del embalaje.' },
    { question: '¿Cómo inicio una devolución?', answer: 'Escribe a support@knitnprint.com con el número de pedido y el motivo de la devolución, y espera nuestras instrucciones antes de enviar el producto.' },
    { question: '¿Quién paga los gastos de devolución?', answer: 'Normalmente los paga el cliente, salvo cuando la devolución se debe a un error de KnitnPrint o a un defecto confirmado del producto.' },
    { question: '¿Cuándo recibiré el reembolso?', answer: 'Una vez recibida e inspeccionada una devolución admisible, el reembolso se procesará en un plazo máximo de 14 días.' },
  ],
}
