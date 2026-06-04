package hianova.stockgo;

import java.nio.file.Files;
import java.nio.file.Paths;
import java.time.LocalDate;
import java.time.Period;
import java.util.HashMap;
import java.util.regex.Pattern;
import java.util.stream.IntStream;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

public class Manager extends Config {

  public Manager() throws Exception {
  }

  public void addConfig(HashMap<String, String> listIn) throws Exception {
    if (CONFIG_TITLE.contains(listIn.get("title"))) {
      System.out.println("Title exists");
      return;
    }
    CONFIG_TITLE.add(listIn.get("title"));
    CONFIG_URL.add(listIn.get("URL"));
    CONFIG_FOLDER.add(listIn.get("title"));
    CONFIG_LABEL.add(listIn.get("label"));
    CONFIG_STATUS.add(listIn.get("status"));
    download(listIn.get("URL"));
    CONFIG_STATUS.set(CONFIG_STATUS.size(), LocalDate.now().format(UNI_DATE));
    syncConfig();
    System.out.println(listIn.get("title") + "added");
  }

  public void deleteConfig(int numIn) throws Exception {
    CONFIG_TITLE.remove(numIn);
    CONFIG_URL.remove(numIn);
    CONFIG_FOLDER.remove(numIn);
    CONFIG_LABEL.remove(numIn);
    CONFIG_STATUS.remove(numIn);
    syncConfig();
    System.out.println(numIn + " deleted");
  }

  public void update() {
    IntStream.range(0, CONFIG_STATUS.size()).forEach(next -> {
      if (Period.between(LocalDate.parse(CONFIG_STATUS.get(next), UNI_DATE),
          LocalDate.now()).getDays() > 1) {
        try {
          download(CONFIG_URL.get(next));
          CONFIG_STATUS.set(next, LocalDate.now().format(UNI_DATE));
          syncConfig();
        } catch (Exception e) {
          System.out.println("Update has been suspended: " + e);
        }
      }
    });
    System.out.println("Files are up to update");
  }

  public void download(String urlIn) throws Exception {
    var dir = Paths.get("downloads", CONFIG_FOLDER.get(CONFIG_URL.indexOf(urlIn)));
    var postRgx = Pattern.compile("@Post:");

    Files.createDirectory(dir);
    streamNum(urlIn, "").forEach(nextNum -> {
      try {
        streamDate(urlIn, "").forEach(nextDate -> {
          try {
            var url = urlIn.replace("@date",
                toOrigin(nextDate, urlIn)).replace("@num", nextNum).split("@Post:");
            var crawl = new Crawl(url[0]);
            var path = String.format("%s%s%s%s.txt", dir, LIB.URLToName(url[0]),
                (NUM_RGX.matcher(urlIn).find() ? "_" + nextNum : ""),
                (DATE_RGX.matcher(urlIn).find() ? "_" + nextDate : ""));
            if (postRgx.matcher(urlIn).find()) {
              crawl.setPost(url[1]);
            }
            crawl.setPath(path);
            crawl.save();
            Thread.sleep((long) (Math.random() * 5000));// be gentle
          } catch (Exception e) {
            System.out.println("Time iterator stopped: " + e);
          }
        });
      } catch (Exception e) {
        System.out.println("Number iterator stopped: " + e);
      }
    });
  }

  public void addRelay(String nameIn, HashMap<String, String> listIn) throws Exception {
    var path = Paths.get("downloads", "relay.json");
    var json = (ObjectNode) new ObjectMapper().readTree(path.toFile());
    var tmp = json.putObject(nameIn);
    listIn.entrySet().forEach(next -> {
      tmp.put(next.getKey(), next.getValue());
    });
    Files.writeString(path, json.toPrettyString());
  }

  public void deleteRelay(String nameIn) throws Exception {
    var path = Paths.get("downloads", "relay.json");
    var json = (ObjectNode) new ObjectMapper().readTree(path.toFile());
    json.remove(nameIn);
    Files.writeString(path, json.toPrettyString());
  }
}
