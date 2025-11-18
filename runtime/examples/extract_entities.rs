use anyhow::{Context, Result};
use dotenvy::dotenv;
use runtime::ai::{
    responses::ResponsesClient,
    schemas::{EntitiesRelationships, entities_relationships_schema},
};
use std::{env, sync::Arc};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().expect("Couln't load .env file");
    let api_key = env::var("OPENAI_API_KEY").context("openai aapi key not set")?;
    let system_prompt =
        String::from("You are a helpful assistant who extracts entities from a given chunk.");
    let user_prompt = match env::args().nth(1) {
        Some(arg) => arg,
        None => ", metabolism and nutritional implications. Front Physiol. 2017;8(8):902.  \n51. Keppel Hesselink JM, de Boer T, Witkamp RF. Palmitoylethanolamide: a natural body-own anti-inflammatory agent, effective and safe against influenza and common cold. Int J Inflam. 2013;2013:151028.  \n52. Ueda N. Endocannabinoid hydrolases. Prostaglandins Other Lipid Mediat. 2002;68-69:521-34.  \n53. Wang J, Yu Y, Song Z, Han D, Zhang J, Chen L, et al. A high fat diet with a high C18:0/C16:0 ratio induced worse metabolic and transcriptomic profiles in C57BL/6 mice. Lipids in Health and Disease. 2020;19:172.  \n54. Donnelly KL, Smith CI, Schwarzenberg SJ, Jessurun J, Boldt MD, Parks EJ. Sources of fatty acids stored in liver and secreted via lipoproteins in patients with nonalcoholic fatty liver disease. J Clin Invest. 2005;115:1343-51.  \n55. Clayton P, Hill M, Bogoda N, Subah S, Venkatesh R. Palmitoylethanolamide: a natural compound for health management. Int J Mol Sci. 2021;22:5305.  \n56. Ambrosino P, Soldovieri MV, Russo C, Taglialatela M. Activation and desensitization of TRPV1 channels in sensory neurons by the PPARα agonist palmitoylethanolamide. Br J Pharmacol. 2013;168:1430-44.  \n57. Petrosino S, Schiano Moriello A, Verde R, Allarà M, Imperatore R, Ligresti A, et al. Palmitoylethanolamide counteracts substance P-induced mast cell activation in vitro by stimulating diacylglycerol lipase activity. J Neuroinflammation. 2019;16:274.  \n58. LoVerme J, La Rana G, Russo R, Calignano A, Piomelli D. The search for the palmitoylethanolamide receptor. Life Sci. 2005;77:1685-98.  \n59. Petrosino S, Di".to_owned()
    };
    let client = Arc::new(ResponsesClient::new(api_key, None));
    let entities = entities_relationships_schema();
    let model_str = "gpt-5-mini".to_owned();
    let response: EntitiesRelationships = client
        .responses_structured(
            &model_str,
            &system_prompt,
            &user_prompt,
            None,
            "entities",
            entities,
            true,
        )
        .await?;
    println!("{:?}", response);
    Ok(())
}
