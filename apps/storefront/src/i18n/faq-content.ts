export type FaqGroup = {
  id: string
  title: string
  questions: { question: string; answer: string }[]
}

export const faqGroupsPt: FaqGroup[] = [
  { id: 'personalisation', title: 'Personalização', questions: [
    { question: 'Que produtos posso personalizar?', answer: 'Pode personalizar têxteis, garrafas, mochilas, acessórios e presentes selecionados. As opções disponíveis são apresentadas em cada página de produto.' },
    { question: 'O que posso acrescentar a um produto?', answer: 'Consoante a peça, poderá acrescentar um nome, iniciais, uma frase curta, uma ilustração ou outro elemento gráfico.' },
    { question: 'Posso utilizar o meu próprio design ou ilustração?', answer: 'Sim, desde que o ficheiro tenha qualidade suficiente para a técnica escolhida e tenha autorização para utilizar o design.' },
    { question: 'Vou ver o design antes da produção?', answer: 'Quando o serviço inclui uma maquete digital, pediremos que confirme a composição, a escala e a posição antes do início da produção.' },
    { question: 'Posso alterar a personalização depois de encomendar?', answer: 'Contacte-nos o mais depressa possível. Poderá ser possível fazer alterações antes do início da produção, mas não podemos garanti-las quando a peça já está a ser produzida.' },
  ] },
  { id: 'products', title: 'Produtos e materiais', questions: [
    { question: 'Que materiais utilizam?', answer: 'Os materiais variam consoante o produto e são descritos na respetiva página. Se precisar de ajuda para escolher, contacte-nos antes de encomendar.' },
    { question: 'As cores são exatamente iguais às apresentadas online?', answer: 'Procuramos representar as cores com rigor, mas os ecrãs, lotes de produção e materiais naturais podem originar pequenas variações.' },
    { question: 'Como escolho o tamanho certo?', answer: 'Consulte as medidas no guia de tamanhos do produto e compare-as com uma peça semelhante que já tenha.' },
    { question: 'Os produtos são feitos à mão?', answer: 'A personalização e o acabamento incluem processos manuais, mesmo quando utilizamos equipamento especializado para garantir um resultado consistente.' },
    { question: 'Posso pedir um produto que não esteja disponível no site?', answer: 'Pode contactar-nos com a sua ideia. Para quantidades maiores ou projetos empresariais, utilize o formulário de proposta B2B.' },
  ] },
  { id: 'orders', title: 'Encomendas e pagamentos', questions: [
    { question: 'Como faço uma encomenda?', answer: 'Escolha o produto, selecione as opções disponíveis, acrescente os dados de personalização e reveja tudo cuidadosamente antes do checkout.' },
    { question: 'Que métodos de pagamento aceitam?', answer: 'Os métodos disponíveis para a sua encomenda e localização são apresentados de forma segura durante o checkout.' },
    { question: 'O pagamento é seguro?', answer: 'Os pagamentos são tratados por prestadores especializados. A KnitnPrint não guarda os dados completos do seu cartão.' },
    { question: 'Posso utilizar mais do que um código de desconto?', answer: 'Salvo indicação em contrário, os códigos de desconto não são acumuláveis. Introduza o código no checkout para confirmar se é aplicável.' },
    { question: 'Onde encontro a confirmação da encomenda?', answer: 'Enviamo-la para o email utilizado no checkout. Se não estiver na caixa de entrada, verifique o spam antes de nos contactar.' },
  ] },
  { id: 'delivery', title: 'Produção e entrega', questions: [
    { question: 'Quanto tempo demora a produção?', answer: 'O prazo depende do produto, quantidade e complexidade da personalização. A estimativa fornecida com a encomenda aplica-se antes do tempo de envio.' },
    { question: 'A produção está incluída na estimativa de entrega?', answer: 'A produção e o envio são etapas diferentes. Um artigo personalizado tem primeiro de ser produzido e verificado antes de ser entregue à transportadora.' },
    { question: 'Como saberei que a encomenda foi enviada?', answer: 'Enviaremos uma confirmação de expedição e, sempre que disponível, a informação necessária para acompanhar a entrega.' },
    { question: 'Para onde fazem entregas?', answer: 'Os destinos disponíveis são apresentados no checkout. Contacte-nos antes de encomendar se o seu destino não estiver indicado.' },
    { question: 'O que devo fazer se a encomenda chegar danificada?', answer: 'Contacte-nos rapidamente com o número da encomenda e fotografias nítidas da embalagem, do pacote e dos artigos afetados para avaliarmos o sucedido.' },
  ] },
  { id: 'returns', title: 'Devoluções e reembolsos', questions: [
    { question: 'Posso devolver produtos personalizados?', answer: 'Em geral, os produtos personalizados não podem ser devolvidos por mudança de opinião. Isto não afeta os seus direitos quando um artigo tem defeito, está danificado ou difere do acordado.' },
    { question: 'E se existir um erro na personalização?', answer: 'Envie-nos o número da encomenda e fotografias nítidas. Compararemos o artigo final com a personalização confirmada e indicaremos o próximo passo.' },
    { question: 'Posso devolver um produto não personalizado?', answer: 'Os produtos não personalizados elegíveis podem ser devolvidos nas condições e prazos indicados na nossa Política de Devoluções.' },
    { question: 'Quem paga os custos da devolução?', answer: 'Depende do motivo da devolução. Consulte a Política de Devoluções ou contacte-nos antes de enviar qualquer artigo.' },
    { question: 'Quando receberei o reembolso?', answer: 'Depois de recebermos e verificarmos uma devolução elegível, o reembolso é processado dentro do prazo indicado na Política de Devoluções.' },
  ] },
  { id: 'b2b', title: 'Encomendas B2B', questions: [
    { question: 'Trabalham com empresas e associações?', answer: 'Sim. Aceitamos pedidos de empresas, associações, escolas, clubes, equipas e organizadores de eventos.' },
    { question: 'Existe uma quantidade mínima?', answer: 'Os projetos corporativos estão sujeitos a quantidades mínimas, que variam consoante o produto e a técnica de personalização.' },
    { question: 'Podem ajudar-nos a escolher os produtos certos?', answer: 'Sim. Podemos analisar a finalidade, quantidade, orçamento e acabamento antes de prepararmos uma proposta personalizada.' },
    { question: 'Fornecem uma maquete digital?', answer: 'Preparamos uma maquete digital para poder rever a composição, a escala e a posição antes de aprovar a produção.' },
    { question: 'Que formatos de logótipo aceitam?', answer: 'Pode enviar ficheiros AI, EPS, PDF, SVG, PNG ou JPG. Os formatos vetoriais costumam garantir o melhor resultado.' },
    { question: 'Como posso pedir uma proposta?', answer: 'Preencha o formulário da página B2B com os dados da empresa, tipo de produto, quantidade estimada e ficheiro do logótipo.' },
  ] },
  { id: 'care', title: 'Cuidados', questions: [
    { question: 'Como devo lavar roupa personalizada?', answer: 'Salvo indicação contrária, lave a peça do avesso, a baixa temperatura, com detergente suave e cores semelhantes.' },
    { question: 'A roupa personalizada pode ir à máquina de secar?', answer: 'A secagem natural costuma ser a opção mais delicada e ajuda a preservar tanto a peça como a personalização.' },
    { question: 'Posso passar a ferro sobre a área personalizada?', answer: 'Evite passar diretamente sobre o design. Vire a peça do avesso e siga a etiqueta de cuidados.' },
    { question: 'Como devo limpar garrafas e acessórios?', answer: 'Os cuidados dependem do material e acabamento. Siga as instruções do produto e evite utensílios de limpeza abrasivos.' },
    { question: 'Como posso prolongar a duração da personalização?', answer: 'Trate a peça com cuidado, siga as instruções de lavagem ou limpeza e guarde-a longe de calor, humidade e luz solar direta excessivos.' },
  ] },
]

export const faqGroupsEs: FaqGroup[] = [
  { id: 'personalisation', title: 'Personalización', questions: [
    { question: '¿Qué productos puedo personalizar?', answer: 'Puedes personalizar textiles, botellas, mochilas, accesorios y regalos seleccionados. Las opciones disponibles aparecen en cada página de producto.' },
    { question: '¿Qué puedo añadir a un producto?', answer: 'Según la pieza, podrás añadir un nombre, iniciales, una frase corta, una ilustración u otro elemento gráfico.' },
    { question: '¿Puedo utilizar mi propio diseño o ilustración?', answer: 'Sí, siempre que el archivo tenga calidad suficiente para la técnica elegida y tengas permiso para utilizar el diseño.' },
    { question: '¿Veré el diseño antes de la producción?', answer: 'Cuando el servicio incluya una maqueta digital, te pediremos que confirmes la composición, la escala y la posición antes de producir.' },
    { question: '¿Puedo cambiar la personalización después de pedir?', answer: 'Contacta con nosotros cuanto antes. Podremos realizar cambios antes de comenzar la producción, pero no podemos garantizarlos cuando la pieza ya se está produciendo.' },
  ] },
  { id: 'products', title: 'Productos y materiales', questions: [
    { question: '¿Qué materiales utilizáis?', answer: 'Los materiales varían según el producto y se describen en su página. Si necesitas ayuda para elegir, contacta con nosotros antes de pedir.' },
    { question: '¿Los colores son exactamente iguales a los mostrados online?', answer: 'Procuramos representar los colores con precisión, pero las pantallas, los lotes de producción y los materiales naturales pueden producir pequeñas variaciones.' },
    { question: '¿Cómo elijo la talla correcta?', answer: 'Consulta las medidas de la guía de tallas del producto y compáralas con una prenda similar que ya tengas.' },
    { question: '¿Los productos están hechos a mano?', answer: 'La personalización y el acabado incluyen procesos manuales, incluso cuando utilizamos equipos especializados para lograr un resultado uniforme.' },
    { question: '¿Puedo solicitar un producto que no aparezca en la web?', answer: 'Puedes contactar con nosotros y contarnos tu idea. Para cantidades mayores o proyectos empresariales, utiliza el formulario B2B.' },
  ] },
  { id: 'orders', title: 'Pedidos y pagos', questions: [
    { question: '¿Cómo hago un pedido?', answer: 'Elige el producto, selecciona las opciones disponibles, añade los datos de personalización y revisa todo cuidadosamente antes de pagar.' },
    { question: '¿Qué métodos de pago aceptáis?', answer: 'Los métodos disponibles para tu pedido y ubicación se muestran de forma segura durante el pago.' },
    { question: '¿El pago es seguro?', answer: 'Los pagos los gestionan proveedores especializados. KnitnPrint no guarda los datos completos de tu tarjeta.' },
    { question: '¿Puedo utilizar más de un código de descuento?', answer: 'Salvo que la promoción indique lo contrario, los códigos no se pueden combinar. Introduce el código al pagar para confirmar si se aplica.' },
    { question: '¿Dónde encuentro la confirmación del pedido?', answer: 'La enviamos al correo utilizado al pagar. Si no aparece en tu bandeja de entrada, revisa la carpeta de spam antes de contactarnos.' },
  ] },
  { id: 'delivery', title: 'Producción y entrega', questions: [
    { question: '¿Cuánto tarda la producción?', answer: 'El plazo depende del producto, la cantidad y la complejidad de la personalización. La estimación del pedido se aplica antes del tiempo de envío.' },
    { question: '¿La producción está incluida en la estimación de entrega?', answer: 'La producción y el envío son etapas distintas. Primero debemos crear y revisar el artículo personalizado antes de entregarlo al transportista.' },
    { question: '¿Cómo sabré que mi pedido ha sido enviado?', answer: 'Enviaremos una confirmación de envío y, cuando esté disponible, la información necesaria para seguir la entrega.' },
    { question: '¿Dónde realizáis entregas?', answer: 'Los destinos disponibles aparecen durante el pago. Contacta con nosotros antes de pedir si tu destino no figura en la lista.' },
    { question: '¿Qué hago si el paquete llega dañado?', answer: 'Contacta con nosotros rápidamente con el número de pedido y fotos claras del paquete, el embalaje y los artículos afectados para que podamos evaluarlo.' },
  ] },
  { id: 'returns', title: 'Devoluciones y reembolsos', questions: [
    { question: '¿Puedo devolver productos personalizados?', answer: 'Por lo general, los productos personalizados no pueden devolverse por un cambio de opinión. Esto no afecta a tus derechos si un artículo es defectuoso, está dañado o no coincide con lo acordado.' },
    { question: '¿Qué ocurre si hay un error en la personalización?', answer: 'Envíanos el número de pedido y fotos claras. Compararemos el artículo terminado con la personalización confirmada y te indicaremos el siguiente paso.' },
    { question: '¿Puedo devolver un producto no personalizado?', answer: 'Los productos no personalizados elegibles pueden devolverse según las condiciones y plazos de nuestra Política de Devoluciones.' },
    { question: '¿Quién paga los gastos de devolución?', answer: 'Depende del motivo. Consulta la Política de Devoluciones o contacta con nosotros antes de enviar un artículo.' },
    { question: '¿Cuándo recibiré el reembolso?', answer: 'Una vez recibida y revisada una devolución elegible, procesaremos el reembolso dentro del plazo indicado en la Política de Devoluciones.' },
  ] },
  { id: 'b2b', title: 'Pedidos B2B', questions: [
    { question: '¿Trabajáis con empresas y asociaciones?', answer: 'Sí. Aceptamos solicitudes de empresas, asociaciones, colegios, clubes, equipos y organizadores de eventos.' },
    { question: '¿Existe una cantidad mínima?', answer: 'Los proyectos corporativos están sujetos a cantidades mínimas que varían según el producto y la técnica de personalización.' },
    { question: '¿Podéis ayudarnos a elegir los productos adecuados?', answer: 'Sí. Podemos valorar la finalidad, la cantidad, el presupuesto y el acabado antes de preparar una propuesta personalizada.' },
    { question: '¿Proporcionáis una maqueta digital?', answer: 'Preparamos una maqueta digital para que puedas revisar la composición, la escala y la posición antes de aprobar la producción.' },
    { question: '¿Qué formatos de logotipo aceptáis?', answer: 'Puedes enviar archivos AI, EPS, PDF, SVG, PNG o JPG. Los formatos vectoriales suelen ofrecer el mejor resultado.' },
    { question: '¿Cómo puedo solicitar una propuesta?', answer: 'Completa el formulario de la página B2B con los datos de la empresa, el tipo de producto, la cantidad estimada y el archivo del logotipo.' },
  ] },
  { id: 'care', title: 'Cuidados', questions: [
    { question: '¿Cómo debo lavar la ropa personalizada?', answer: 'Salvo que las instrucciones indiquen lo contrario, lava la prenda del revés, a baja temperatura, con detergente suave y colores similares.' },
    { question: '¿La ropa personalizada puede ir a la secadora?', answer: 'El secado natural suele ser la opción más delicada y ayuda a conservar tanto la prenda como la personalización.' },
    { question: '¿Puedo planchar sobre la zona personalizada?', answer: 'Evita planchar directamente sobre el diseño. Dale la vuelta a la prenda y sigue su etiqueta de cuidados.' },
    { question: '¿Cómo debo limpiar botellas y accesorios?', answer: 'Los cuidados dependen del material y el acabado. Sigue las instrucciones del producto y evita utensilios de limpieza abrasivos.' },
    { question: '¿Cómo puedo hacer que la personalización dure más?', answer: 'Trata la pieza con cuidado, sigue las instrucciones de lavado o limpieza y guárdala lejos del calor, la humedad y la luz solar directa excesivos.' },
  ] },
]
