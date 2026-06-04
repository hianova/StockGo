package hianova.stockgo;

import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.HashMap;
import java.util.Optional;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

import io.ipfs.api.IPFS;
import io.ipfs.api.NamedStreamable.ByteArrayWrapper;
import io.ipfs.cid.Cid;

public class IPFSLayer extends Config {

  protected IPFS ipfs;
  protected HashMap<String, String> user;
  protected ObjectNode profile;

  public IPFSLayer() throws Exception {
    ipfs = new IPFS("/ip4/127.0.0.1/tcp/5001");
    var json = new ObjectMapper().readTree(Files.readAllBytes(
        Paths.get("user_data.json"))).get("user");
    json.fields().forEachRemaining(next -> {
      user.put(next.getKey(), next.getValue().asText());
    });
    var tmp = ipfs.name.resolve(Cid.decode(user.get("ipns")));
    profile = (ObjectNode) new ObjectMapper().readTree(tmp);
  }

  public String getProfile() throws Exception {
     return profile.toPrettyString();
  }

  public void syncIpns() throws Exception {
    var tmp = ipfs.add(new ByteArrayWrapper(profile.toPrettyString().getBytes()));
    ipfs.name.publish(tmp.get(0).hash, Optional.of(user.get("key")));
    user.put("ipns", tmp.get(0).hash.toString());
    Files.writeString(Paths.get("user_data.json"), "UTF-8");
  }

  public void delete(String cidIn) throws Exception {
    profile.remove("/stockgo/" + cidIn);
    syncIpns();
  }

  public String share(int numIn) throws Exception {
    String out;
    var map = new HashMap<String, String>();
    var json = new ObjectMapper().createObjectNode();
    var files = json.putObject("files");
    var config = json.putObject("config");

    map.put("title", CONFIG_TITLE.get(numIn));
    map.put("URL", CONFIG_URL.get(numIn));
    map.put("label", CONFIG_LABEL.get(numIn));
    map.put("status", CONFIG_STATUS.get(numIn));
    map.entrySet().forEach(next -> {
      config.put(next.getKey(), next.getValue());
    });
    Files.walk(Paths.get("downloads", map.get("title"))).forEach(
        next -> {
          try {
            var tmp = ipfs.add(new ByteArrayWrapper(Files.readAllBytes(next)));
            files.put(next.toString(), tmp.get(0).hash.toString());
          } catch (Exception e) {
            System.out.println("ipfs can't add file");
          }
        });
    out = ipfs.add(new ByteArrayWrapper(json.toPrettyString().getBytes()))
        .get(0).hash.toString();
    return out;
  }

  public void add(String cidIn) throws Exception {
    var map = new HashMap<String, String>();
    var json = new ObjectMapper().readTree(ipfs.dag.get(Cid.decode(cidIn)));
    var files = json.get("files");
    var congig = json.get("config");

    files.fields().forEachRemaining(next -> {
      map.put(next.getKey(), next.getValue().asText());
    });
    congig.fields().forEachRemaining(next -> {
      map.put(next.getKey(), next.getValue().asText());
    });
    CONFIG_TITLE.add(map.get("title"));
    CONFIG_URL.add(map.get("URL"));
    CONFIG_LABEL.add(map.get("label"));
    CONFIG_STATUS.add(map.get("status"));
    syncConfig();
  }

  public void postArticle(String bodyIn, boolean privateIn) throws Exception {
    var ele = new Element();
    var map = new HashMap<String, String>();

    if (privateIn) {
      ele.chat();
    }
    map.put("body", bodyIn);
    ele.stdUnit(map);
    ele.post();
    System.out.println("article post");
  }
}
