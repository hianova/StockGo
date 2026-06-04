package hianova.stockgo;

import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Random;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

public class Crawl {

  private final Lib LIB;
  private final String TAG;
  private final String CODEC;
  private final HttpURLConnection HTTP;
  private Path path;

  public Crawl(String urlIn) throws Exception {
    String[] ua;
    var uaPath = Paths.get("userAgent.txt");
    var url = new URL(urlIn.replace("https", "http"));
    LIB = new Lib();
    TAG = url.getHost().replace(".", "_");
    CODEC = LIB.tag(TAG + "/codec");
    HTTP = (HttpURLConnection) url.openConnection();
    path = Paths.get("downloads", LIB.URLToName(url.toString()) + ".txt");

    if (Files.exists(uaPath)) {
      ua = Files.readString(uaPath).split("\n");
    } else {
      ua = new String[] {
          "Opera/9.64 (Windows NT 6.0; U; pl) Presto/2.1.1",
          "Mozilla/1.22 (compatible; MSIE 10.0; Windows 3.1)",
          "Mozilla/4.0(compatible; MSIE 7.0b; Windows NT 6.0)"
      };
    }
    HTTP.setRequestProperty("Host", url.getHost());
    HTTP.setRequestProperty("Accept", "*/*");
    HTTP.setRequestProperty("Origin", url.toString());
    HTTP.setInstanceFollowRedirects(true);
    HTTP.setRequestProperty("Referer", url.toString());
    HTTP.setRequestProperty("Connection", "keep-alive");
    HTTP.setRequestProperty("X-Requested-With", "XMLHttpRequest");
    HTTP.setRequestProperty("Accept-Language", "zh-TW,zh-Hant;q=0.9");
    HTTP.setRequestProperty("User-Agent", ua[new Random().nextInt(ua.length)]);
    HTTP.setRequestProperty("Content-Type", "application/x-www-form-urlencoded; charset=" + CODEC);
  }

  public void setPost(String formIn) throws Exception {
    HTTP.setDoOutput(true);
    HTTP.setRequestMethod("POST");
    HTTP.getOutputStream().write(formIn.getBytes());
  }

  public void setPath(String pathIn) {
    path = Paths.get(pathIn);
  }

  public String save() throws Exception {
    var data = new String(HTTP.getInputStream().readAllBytes(), CODEC);

    if (LIB.isHTML(data)) {
      data = String.format("%s<tag>%s</tag>", data, TAG);
    } else if (LIB.isJSON(data)) {
      var tmp = (ObjectNode) new ObjectMapper().readTree(data);
      tmp.put("tag", TAG);
      data = tmp.toPrettyString();
    }
    HTTP.disconnect();
    Files.createFile(path);
    Files.writeString(path, data);
    System.out.println(path + " added");
    return data;
  }
}