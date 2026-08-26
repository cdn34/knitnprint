export type LegalBlock =
  | { type: 'p'; text: string }
  | { type: 'ul' | 'ol'; items: string[] }

export type LegalSection = { number: string; title: string; blocks: LegalBlock[] }

export const termsPt: LegalSection[] = [
  { number: '02', title: 'Objeto e âmbito', blocks: [
    { type: 'p', text: 'A KnitnPrint é uma loja online sediada em Portugal que comercializa produtos personalizados e outros artigos, procurando responder às necessidades dos seus clientes em diversas áreas. Os presentes Termos e Condições regulam o acesso, navegação e utilização do website [site], bem como as condições aplicáveis à compra de produtos através da loja online.' },
    { type: 'p', text: 'Ao utilizar este website ou efetuar uma encomenda, aceita integralmente os presentes Termos e Condições, bem como a Política de Privacidade e a Política de Cookies.' },
    { type: 'p', text: 'Se não concordar com os termos aqui descritos, deverá abster-se de utilizar este website.' },
  ] },
  { number: '03', title: 'Alterações aos Termos e Condições', blocks: [
    { type: 'p', text: 'A KnitnPrint reserva-se o direito de alterar os presentes Termos e Condições a qualquer momento. As alterações produzem efeitos após a sua publicação no website.' },
  ] },
  { number: '04', title: 'Utilização do website', blocks: [
    { type: 'p', text: 'Ao aceder e utilizar este website, compromete-se a:' },
    { type: 'ul', items: ['Utilizar a plataforma apenas para consultas e encomendas legítimas;', 'Não efetuar encomendas falsas, enganosas ou fraudulentas;', 'Fornecer informações completas, exatas e atualizadas, incluindo morada, email e quaisquer outros contactos necessários.'] },
    { type: 'p', text: 'Sempre que existam motivos razoáveis para suspeitar de utilização fraudulenta, a empresa poderá cancelar a encomenda e, quando aplicável, comunicar a situação às autoridades competentes.' },
    { type: 'p', text: 'Ao efetuar uma compra através deste website, confirma que tem pelo menos 18 anos e capacidade legal para celebrar contratos.' },
  ] },
  { number: '05', title: 'Disponibilidade dos produtos', blocks: [
    { type: 'p', text: 'Todos os produtos apresentados no website estão sujeitos à disponibilidade de stock.' },
    { type: 'p', text: 'Se, após a confirmação da encomenda, algum produto adquirido não estiver disponível, o cliente será informado e poderá optar por:' },
    { type: 'ul', items: ['Aguardar a reposição do produto;', 'Escolher um artigo alternativo;', 'Solicitar o reembolso do valor pago.'] },
  ] },
  { number: '06', title: 'Processo de compra', blocks: [
    { type: 'p', text: 'Para efetuar uma encomenda, deverá:' },
    { type: 'ol', items: ['Selecionar os produtos que pretende adquirir;', 'Adicioná-los ao carrinho de compras;', 'Fornecer os dados de faturação e entrega;', 'Escolher um método de pagamento;', 'Confirmar a encomenda.'] },
    { type: 'p', text: 'Após concluir a compra, o cliente receberá um email a confirmar a receção da encomenda.' },
    { type: 'p', text: 'O contrato só se considera celebrado quando for enviado o email de confirmação de expedição da encomenda.' },
  ] },
  { number: '07', title: 'Preços', blocks: [
    { type: 'p', text: 'Todos os preços apresentados no website incluem IVA à taxa legal aplicável, salvo indicação expressa em contrário.' },
    { type: 'p', text: 'Os eventuais custos de entrega são apresentados antes da conclusão e confirmação da compra. São calculados de acordo com as tarifas dos nossos transportadores parceiros e dependem do peso, dimensão e destino da encomenda.' },
    { type: 'p', text: 'A empresa pode atualizar ou alterar os preços dos produtos a qualquer momento, sem afetar encomendas já devidamente confirmadas.' },
    { type: 'p', text: 'Se for identificado um erro manifesto no preço, o cliente será informado e poderá optar por prosseguir com a encomenda ao preço correto ou cancelá-la.' },
  ] },
  { number: '08', title: 'Métodos de pagamento', blocks: [
    { type: 'p', text: 'Os pagamentos são efetuados por cartão de crédito através da plataforma Stripe.' },
  ] },
  { number: '09', title: 'Entrega', blocks: [
    { type: 'p', text: 'Os prazos de entrega podem variar devido a requisitos logísticos e de personalização.' },
    { type: 'p', text: 'A entrega considera-se concluída quando o cliente, ou alguém em seu nome, recebe a encomenda.' },
  ] },
  { number: '10', title: 'Direito de livre resolução', blocks: [
    { type: 'p', text: 'Nos termos do Decreto-Lei n.º 24/2014, os consumidores dispõem de 14 dias para resolver o contrato sem necessidade de indicar o motivo.' },
    { type: 'p', text: 'Este prazo começa na data em que o consumidor, ou um terceiro por si indicado, recebe o produto.' },
    { type: 'p', text: 'Para exercer o direito de livre resolução, o cliente deve comunicar a sua decisão por email para [email].' },
  ] },
  { number: '11', title: 'Devoluções', blocks: [
    { type: 'p', text: 'Os artigos devolvidos devem cumprir as seguintes condições:' },
    { type: 'ul', items: ['Estar em perfeito estado;', 'Não apresentar sinais de utilização;', 'Ser devolvidos na embalagem original;', 'Incluir o respetivo comprovativo de compra.'] },
    { type: 'p', text: 'Os custos de devolução são suportados pelo cliente, salvo quando a devolução resulte de defeito do produto ou erro de entrega.' },
    { type: 'p', text: 'Os reembolsos serão processados no prazo máximo de 14 dias e, sempre que possível, através do mesmo método de pagamento utilizado na compra.' },
    { type: 'p', text: 'Os produtos personalizados ou produzidos no âmbito de encomendas de grande quantidade não podem ser devolvidos, salvo em caso de defeito, erro de produção ou desconformidade com a encomenda efetuada.' },
    { type: 'p', text: 'A KnitnPrint reserva-se o direito de recusar devoluções que não cumpram os critérios aqui descritos.' },
    { type: 'p', text: 'Se receber um produto danificado ou defeituoso, contacte-nos o mais rapidamente possível através de [email].' },
  ] },
  { number: '12', title: 'Garantia', blocks: [
    { type: 'p', text: 'Todos os produtos comercializados beneficiam da garantia legal aplicável aos bens de consumo, nos termos da legislação portuguesa.' },
    { type: 'p', text: 'Para produtos novos, os consumidores beneficiam de um prazo de garantia de três anos, nos termos da lei.' },
    { type: 'p', text: 'A garantia não abrange situações resultantes de:' },
    { type: 'ul', items: ['Desgaste normal decorrente da utilização do produto;', 'Utilização indevida ou inadequada;', 'Alterações, modificações ou intervenções realizadas pelo cliente;', 'Variações naturais ou pequenas irregularidades inerentes ao processo de produção artesanal e às características da pele natural, como diferenças de tonalidade, textura ou grão, que não são consideradas defeitos de fabrico.'] },
  ] },
  { number: '13', title: 'Proteção de dados', blocks: [
    { type: 'p', text: 'A KnitnPrint compromete-se a proteger a privacidade dos seus clientes e atua em conformidade com o Regulamento Geral sobre a Proteção de Dados (RGPD — UE 2016/679).' },
    { type: 'p', text: 'Os dados recolhidos são utilizados exclusivamente para gerir encomendas e comunicar com os clientes.' },
    { type: 'p', text: 'Os clientes têm o direito de aceder, corrigir ou eliminar os seus dados a qualquer momento.' },
  ] },
  { number: '14', title: 'Livro de Reclamações', blocks: [
    { type: 'p', text: 'Nos termos da legislação aplicável, o Livro de Reclamações Eletrónico está disponível em [complaints].' },
  ] },
  { number: '15', title: 'Lei aplicável', blocks: [
    { type: 'p', text: 'Os presentes Termos e Condições são regidos pela lei portuguesa.' },
    { type: 'p', text: 'Em caso de litígio, será competente o tribunal da comarca da sede da empresa, sem prejuízo das disposições legais aplicáveis aos consumidores.' },
  ] },
]

export const termsEs: LegalSection[] = [
  { number: '02', title: 'Objeto y ámbito', blocks: [
    { type: 'p', text: 'KnitnPrint es una tienda online con sede en Portugal que vende productos personalizados y otros artículos para responder a las necesidades de sus clientes en distintas áreas. Estos Términos y condiciones regulan el acceso, la navegación y el uso de [site], así como las condiciones aplicables a la compra de productos en la tienda online.' },
    { type: 'p', text: 'Al utilizar este sitio web o realizar un pedido, aceptas íntegramente estos Términos y condiciones, así como la Política de privacidad y la Política de cookies.' },
    { type: 'p', text: 'Si no estás de acuerdo con estos términos, debes abstenerte de utilizar el sitio web.' },
  ] },
  { number: '03', title: 'Cambios en los Términos y condiciones', blocks: [{ type: 'p', text: 'KnitnPrint se reserva el derecho a modificar estos Términos y condiciones en cualquier momento. Los cambios serán efectivos desde su publicación en el sitio web.' }] },
  { number: '04', title: 'Uso del sitio web', blocks: [
    { type: 'p', text: 'Al acceder y utilizar este sitio web, te comprometes a:' },
    { type: 'ul', items: ['Utilizar la plataforma únicamente para consultas y pedidos legítimos;', 'No realizar pedidos falsos, engañosos o fraudulentos;', 'Facilitar información completa, exacta y actualizada, incluida la dirección, el correo electrónico y cualquier otro dato de contacto necesario.'] },
    { type: 'p', text: 'Cuando existan motivos razonables para sospechar un uso fraudulento, la empresa podrá cancelar el pedido y, cuando proceda, comunicarlo a las autoridades competentes.' },
    { type: 'p', text: 'Al comprar a través de este sitio web, confirmas que tienes al menos 18 años y capacidad legal para celebrar contratos.' },
  ] },
  { number: '05', title: 'Disponibilidad de los productos', blocks: [
    { type: 'p', text: 'Todos los productos mostrados en el sitio web están sujetos a disponibilidad de existencias.' },
    { type: 'p', text: 'Si, después de confirmar el pedido, algún producto no está disponible, se informará al cliente, que podrá:' },
    { type: 'ul', items: ['Esperar a que se reponga el producto;', 'Elegir un artículo alternativo;', 'Solicitar el reembolso del importe pagado.'] },
  ] },
  { number: '06', title: 'Proceso de compra', blocks: [
    { type: 'p', text: 'Para realizar un pedido, debes:' },
    { type: 'ol', items: ['Seleccionar los productos que deseas comprar;', 'Añadirlos a la cesta;', 'Facilitar los datos de facturación y entrega;', 'Elegir un método de pago;', 'Confirmar el pedido.'] },
    { type: 'p', text: 'Tras finalizar la compra, el cliente recibirá un correo confirmando la recepción del pedido.' },
    { type: 'p', text: 'El contrato solo se considera celebrado cuando se envía el correo de confirmación de expedición.' },
  ] },
  { number: '07', title: 'Precios', blocks: [
    { type: 'p', text: 'Todos los precios mostrados incluyen el IVA al tipo legal aplicable, salvo indicación expresa en contrario.' },
    { type: 'p', text: 'Los posibles gastos de entrega se muestran antes de finalizar y confirmar la compra. Se calculan según las tarifas de nuestros transportistas colaboradores y dependen del peso, tamaño y destino del pedido.' },
    { type: 'p', text: 'La empresa puede actualizar o modificar los precios en cualquier momento, sin afectar a los pedidos ya confirmados.' },
    { type: 'p', text: 'Si se detecta un error manifiesto en el precio, se informará al cliente, que podrá continuar al precio correcto o cancelar el pedido.' },
  ] },
  { number: '08', title: 'Métodos de pago', blocks: [{ type: 'p', text: 'Los pagos se realizan con tarjeta de crédito a través de la plataforma Stripe.' }] },
  { number: '09', title: 'Entrega', blocks: [
    { type: 'p', text: 'Los plazos de entrega pueden variar por requisitos logísticos y de personalización.' },
    { type: 'p', text: 'La entrega se considera realizada cuando el cliente, o una persona que actúe en su nombre, recibe el pedido.' },
  ] },
  { number: '10', title: 'Derecho de desistimiento', blocks: [
    { type: 'p', text: 'De conformidad con el Decreto-Ley portugués n.º 24/2014, los consumidores disponen de 14 días para desistir del contrato sin indicar el motivo.' },
    { type: 'p', text: 'El plazo comienza cuando el consumidor, o un tercero designado por este, recibe el producto.' },
    { type: 'p', text: 'Para ejercer este derecho, el cliente debe comunicar su decisión por correo electrónico a [email].' },
  ] },
  { number: '11', title: 'Devoluciones', blocks: [
    { type: 'p', text: 'Los artículos devueltos deben:' },
    { type: 'ul', items: ['Estar en perfecto estado;', 'No mostrar señales de uso;', 'Devolverse en su embalaje original;', 'Incluir el justificante de compra correspondiente.'] },
    { type: 'p', text: 'Los gastos de devolución corren a cargo del cliente, salvo cuando se deba a un defecto del producto o a un error de entrega.' },
    { type: 'p', text: 'Los reembolsos se procesarán en un plazo máximo de 14 días y, siempre que sea posible, mediante el mismo método de pago utilizado.' },
    { type: 'p', text: 'Los productos personalizados o fabricados como parte de pedidos de gran cantidad no se pueden devolver, salvo por defecto, error de producción o falta de conformidad con el pedido.' },
    { type: 'p', text: 'KnitnPrint se reserva el derecho a rechazar devoluciones que no cumplan estos criterios.' },
    { type: 'p', text: 'Si recibes un producto dañado o defectuoso, contacta con nosotros cuanto antes en [email].' },
  ] },
  { number: '12', title: 'Garantía', blocks: [
    { type: 'p', text: 'Todos los productos están cubiertos por la garantía legal aplicable a los bienes de consumo según la legislación portuguesa.' },
    { type: 'p', text: 'Para los productos nuevos, los consumidores disponen de un periodo de garantía de tres años conforme a la ley.' },
    { type: 'p', text: 'La garantía no cubre situaciones derivadas de:' },
    { type: 'ul', items: ['Desgaste normal por el uso;', 'Uso indebido o inadecuado;', 'Alteraciones, modificaciones o intervenciones realizadas por el cliente;', 'Variaciones naturales o pequeñas irregularidades propias de la producción artesanal y de las características de la piel natural, como diferencias de tono, textura o grano, que no se consideran defectos de fabricación.'] },
  ] },
  { number: '13', title: 'Protección de datos', blocks: [
    { type: 'p', text: 'KnitnPrint protege la privacidad de sus clientes y actúa de conformidad con el Reglamento General de Protección de Datos (RGPD — UE 2016/679).' },
    { type: 'p', text: 'Los datos recogidos se utilizan exclusivamente para gestionar pedidos y comunicarse con los clientes.' },
    { type: 'p', text: 'Los clientes pueden acceder, corregir o eliminar sus datos en cualquier momento.' },
  ] },
  { number: '14', title: 'Libro de reclamaciones', blocks: [{ type: 'p', text: 'De conformidad con la legislación aplicable, el Libro de Reclamaciones Electrónico portugués está disponible en [complaints].' }] },
  { number: '15', title: 'Legislación aplicable', blocks: [
    { type: 'p', text: 'Estos Términos y condiciones se rigen por la legislación portuguesa.' },
    { type: 'p', text: 'En caso de litigio, será competente el tribunal del distrito donde la empresa tenga su domicilio social, sin perjuicio de las disposiciones legales aplicables a los consumidores.' },
  ] },
]
