package hianova.stockgo;

import java.io.FileWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

import com.opencsv.CSVWriter;

public class Selecter extends Config {

  private final String[] URL, NUM, DATE;
  private final ArrayList<String>[] REQ, DATA;
  private boolean checkDate;

  public Selecter(ArrayList<String> cmdIn) throws Exception {
    var sum = 0;
    var size = cmdIn.size();
    var allRgx = Pattern.compile("^ALL$");
    URL = new String[size];
    NUM = new String[size];
    DATE = new String[size];
    REQ = new ArrayList[size];

    IntStream.range(0, size).forEach(nextCmd -> {
      try {
        var cmd = Arrays.stream(cmdIn.get(nextCmd).split("-"))
            .map(String::trim).collect(Collectors.toList());
        var map = relay(cmd.get(0));

        if (map.get("URL").isEmpty()) {
          map.put("URL", cmd.get(0));
        }
        IntStream.range(1, cmd.size()).forEach(next -> {
          var tmp = cmd.get(next).split("=");
          map.put(tmp[0], tmp[1]);
        });
        if (map.get("withdate").contains("fales")) {
          checkDate = false;
        } else {
          checkDate = true;
        }
        URL[nextCmd] = map.get("URL");
        REQ[nextCmd] = new ArrayList<>(
            Arrays.asList(map.get("request").split("\\.")));
        DATE[nextCmd] = map.get("date");
        NUM[nextCmd] = map.get("num");
      } catch (Exception e) {
        System.out.println("relay not found");
      }
    });
    for (var count = 0; count < REQ.length; count++) {
      if (allRgx.matcher(REQ[count].get(0)).find()) {
        try {
          var tmp = Files.readString(Paths.get("downloads",
              CONFIG_FOLDER.get(CONFIG_URL.indexOf(URL[count])), "index.csv"));
          var index = new ArrayList<>(Arrays.asList(tmp.split(",")));
          REQ[count] = index;
        } catch (IOException e) {
          System.out.println("index not found: ALL command not available");
        }
      }
      sum += REQ[count].size() + (checkDate & DATE[count].isEmpty() ? 1 : 0);
    }
    DATA = new ArrayList[sum];
  }

  public ArrayList<String>[] select() throws Exception {
    ArrayList<String>[] out;
    var threads = new Thread[URL.length];

    IntStream.range(0, URL.length).forEach(nextURL -> {
      threads[nextURL] = new Thread(() -> {
        try {
          streamNum(URL[nextURL], NUM[nextURL])
              .forEach(nextNum -> streamDate(URL[nextURL], DATE[nextURL]).forEach(nextDate -> {
                try {
                  var que = new ArrayList<String>();
                  var index = new ArrayList<Integer>();
                  var dir = Paths.get("downloads", CONFIG_FOLDER.get(CONFIG_URL.indexOf(URL[nextURL])));
                  var path = String.format("%s%s%s%s.txt", dir,
                      LIB.URLToName(URL[nextURL].split("@Post:")[0]),
                      (NUM_RGX.matcher(URL[nextURL]).find() ? "_" + nextNum : ""),
                      (DATE_RGX.matcher(URL[nextURL]).find() ? "_" + nextDate : ""));
                  IntStream.range(0, REQ[nextURL].size()).forEach(next -> {
                    var tmp = getCache(String.format("%s-%s", path, REQ[nextURL].get(next)));
                    if (tmp == null) {
                      index.add(next);
                      que.add(REQ[nextURL].get(next));
                    } else {
                      if (checkDate & DATE[nextURL].isEmpty()) {
                        DATA[nextURL + next + 1] = tmp;
                      } else {
                        DATA[nextURL + next] = tmp;
                      }
                    }
                  });
                  var parse = new Parse(path, que).data();
                  IntStream.range(0, index.size()).forEach(next -> {
                    DATA[nextURL + index.get(next)] = parse[next];
                    setCache(String.format("%s-%s", path, REQ[nextURL].get(index.get(next))), parse[next]);
                    if (checkDate & DATE[nextURL].isEmpty()) {
                      DATA[nextURL + index.get(next) + 1] = parse[next];
                    } else {
                      DATA[nextURL + index.get(next)] = parse[next];
                    }
                  });
                } catch (Exception e) {
                  System.out.println("can't parse data " + e);
                }
              }));
        } catch (Exception e) {
          System.out.println("can't select data " + e);
        }
      });
      threads[nextURL].start();
    });
    for (var tmp : threads) {
      tmp.join();
    }
    out = DATA;
    return out;
  }

  public void export(String pathIn, boolean assertHeadIn) throws Exception {
    var checkDone = false;
    var size = Arrays.stream(REQ).mapToInt(ArrayList::size).sum();
    var path = pathIn.isBlank() ? Paths.get("downloads", "export.csv") : Paths.get(pathIn);

    Files.createFile(path);
    try (var file = new CSVWriter(new FileWriter(path.toFile()));) {
      var buff = new String[size];
      for (var reqNum = 0; reqNum < REQ.length; reqNum++) {
        if (checkDate & DATE[reqNum].isEmpty()) {
          REQ[reqNum].add(0, "pathIn");
        }
        for (var cell = 0; cell < REQ[reqNum].size(); cell++) {
          buff[reqNum + cell] = REQ[reqNum].get(cell);
        }
      }
      file.writeNext(buff);
      while (checkDone) {
        for (var dataNum = 0; dataNum < DATA.length; dataNum++) {
          for (var cell = 0; cell < DATA[dataNum].size(); cell++) {
            var tmp = "";
            if (cell > DATA[dataNum].size()) {
              checkDone = false;
            } else {
              DATA[dataNum].get(cell);
              checkDone = true;
            }
            buff[dataNum + cell] = tmp;
          }
        }
        file.writeNext(buff);
      }
    }
  }

  public String backTest(String nameIn) throws Exception {
    var tmp = new BackTest(DATA, nameIn);
    var out = String.format("WinRate: %s\nExpectValue: %s\n%s",
        tmp.getWinRate(), tmp.getExpectValue(), tmp.toString());
    return out;
  }

  public ArrayList<String>[] getRequests() {
    return REQ;
  }
}