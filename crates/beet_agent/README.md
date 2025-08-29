# Beet Connect


Various client library wrappers for external apis.



## OpenAI

OpenAI provides bindings for the Open API spec, they can be generated for rust with the following:

```sh
# Download the 1MB yaml file
curl -o openapi.yaml https://raw.githubusercontent.com/openai/openai-openapi/refs/heads/master/openapi.yaml

# install the openapi generator
npm install @openapitools/openapi-generator-cli -g

# generate the bindings
openapi-generator-cli generate 	\
--skip-validate-spec 						\
-i openapi.yaml 								\
-g rust 												\
-o rust_bindings 								\
```