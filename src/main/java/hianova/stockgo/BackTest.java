package hianova.stockgo;

import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;

import javax.script.ScriptEngineManager;

public class BackTest {

  private ArrayList<String>[] data;
  private ArrayList<Integer> mark;
  private int oddP, oddN, pointP, pointN;

  public BackTest(ArrayList<String>[] dataIn, String nameIn) throws Exception {
    var file = Files.readString(Paths.get("strategy", nameIn + ".js"));
    var js = new ScriptEngineManager().getEngineByName("jsvascript");
    data = dataIn;

    js.put("in", nameIn);
    js.eval(file);
    mark = (ArrayList<Integer>) js.get("out");
    for (var count = 0; count < mark.size(); count += 2) {
      var start = Integer.parseInt(data[0].get(mark.get(count)));
      var end = Integer.parseInt(data[0].get(mark.get(count++)));
      var sum = end - start;
      if (sum > 0) {
        oddP += 1;
        pointP += sum;
      } else {
        oddN += 1;
        pointN += sum;
      }
    }
  }

  public String getWinRate() {
    return String.valueOf(oddP / oddN + oddP);
  }

  public String getExpectValue() {
    var sum = oddN + oddN;
    var out = String.valueOf((pointP * oddP / sum) + (pointN * oddN / sum));
    return out;
  }

  public ArrayList<String>[] getData() {
    return data;
  }

  public ArrayList<Integer> getMark() {
    return mark;
  }

  public double getOddP() {
    return oddP;
  }

  public double getOddN() {
    return oddN;
  }

  public int getPointP() {
    return pointP;
  }

  public int getPointN() {
    return pointN;
  }

  @Override
  public String toString() {
    return "BackTest [oddP=" + oddP + ", oddN=" + oddN +
        ", pointP=" + pointP + ", pointN=" + pointN + "]";
  }

}